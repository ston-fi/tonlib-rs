use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::future::join_all;
use moka::future::Cache;
use parking_lot::RwLock;
use tonlib_core::cell::{ArcCell, BagOfCells};
use tonlib_core::library_helper::TonLibraryError;
use tonlib_core::TonHash;

use super::LibraryProvider;
use crate::client::{TonClient, TonClientInterface};
use crate::tl::TonLibraryId;

#[derive(Clone, Copy)]
pub struct LibraryCacheParams {
    capacity: u64,
    time_to_live: Duration,
    code_extra_libs_capacity: u64,
    code_extra_libs_time_to_idle: Duration,
}

impl Default for LibraryCacheParams {
    fn default() -> Self {
        Self {
            capacity: 300,
            time_to_live: Duration::from_secs(300),
            code_extra_libs_capacity: 1000,
            code_extra_libs_time_to_idle: Duration::from_secs(600),
        }
    }
}

#[derive(Clone)]
pub struct BlockchainLibraryProvider {
    inner: Arc<Inner>,
}

struct Inner {
    client: TonClient,
    libs_cache: Cache<TonHash, ArcCell>,
    code_dyn_libs_cache: moka::sync::Cache<TonHash, Arc<RwLock<HashSet<TonHash>>>>, // code_hash -> set of lib_hashes
}

#[async_trait]
impl LibraryProvider for BlockchainLibraryProvider {
    async fn get_or_load_libs(
        &self,
        lib_ids: HashSet<TonHash>,
    ) -> Result<HashMap<TonHash, ArcCell>, TonLibraryError> {
        self.inner.get_libs(lib_ids).await
    }

    async fn get_or_load_code_libs(
        &self,
        code: TonHash,
    ) -> Result<HashMap<TonHash, ArcCell>, TonLibraryError> {
        let Some(lib_ids) = self
            .inner
            .code_dyn_libs_cache
            .get(&code)
            .map(|x| x.read().clone())
        else {
            return Ok(HashMap::new());
        };
        self.inner.get_libs(lib_ids).await
    }

    fn update_code_libs(&self, code: TonHash, lib_id: TonHash) {
        self.inner
            .code_dyn_libs_cache
            .entry(code)
            .or_default()
            .value()
            .write()
            .insert(lib_id);
    }
}

impl BlockchainLibraryProvider {
    const MAX_LIBS_REQUESTED: usize = 255;

    pub fn new(client: &TonClient, cache_params: Option<LibraryCacheParams>) -> Self {
        Self {
            inner: Arc::new(Inner::new(client, cache_params)),
        }
    }

    pub async fn load_libraries(
        &self,
        hashes: &[TonHash],
    ) -> Result<Vec<ArcCell>, TonLibraryError> {
        let mut results = Vec::new();

        // If hashes exceed MAX_LIBS_REQUESTED, split them into chunks
        for chunk in hashes.chunks(Self::MAX_LIBS_REQUESTED) {
            let mut partial_result = self.inner.load_libraries_impl(chunk).await?;
            results.append(&mut partial_result);
        }
        Ok(results)
    }
}

impl Inner {
    fn new(client: &TonClient, cache_params: Option<LibraryCacheParams>) -> Self {
        let cache_params = cache_params.unwrap_or_default();

        let cache = Cache::builder()
            .max_capacity(cache_params.capacity)
            .time_to_live(cache_params.time_to_live)
            .build();
        let code_extra_libs_cache = moka::sync::Cache::builder()
            .max_capacity(cache_params.code_extra_libs_capacity)
            .time_to_idle(cache_params.code_extra_libs_time_to_idle)
            .build();

        Self {
            client: client.clone(),
            libs_cache: cache,
            code_dyn_libs_cache: code_extra_libs_cache,
        }
    }

    async fn get_libs(
        &self,
        lib_ids: HashSet<TonHash>,
    ) -> Result<HashMap<TonHash, ArcCell>, TonLibraryError> {
        let mut result_libs = HashMap::new();

        let cached_libs_future = lib_ids.into_iter().map(|key| async move {
            let maybe_cached = self.libs_cache.get(&key).await;
            (key, maybe_cached)
        });
        let maybe_cached_libs = join_all(cached_libs_future).await;

        let mut hashes_to_load = vec![];
        for (hash, value) in maybe_cached_libs {
            match value {
                Some(lib) => {
                    result_libs.insert(hash, lib);
                }
                None => {
                    hashes_to_load.push(hash);
                }
            }
        }

        let blockchain_libs = self.load_libraries_impl(hashes_to_load.as_slice()).await?;
        result_libs.extend(blockchain_libs.iter().map(|l| (l.cell_hash(), l.clone())));

        self.insert_to_lib_cache(blockchain_libs).await?;
        Ok(result_libs)
    }

    async fn insert_to_lib_cache(&self, libs: Vec<ArcCell>) -> Result<(), TonLibraryError> {
        let mut cache_insert_futures = Vec::with_capacity(libs.len());
        for lib in libs {
            cache_insert_futures.push(self.libs_cache.insert(lib.cell_hash(), lib.clone()));
        }
        join_all(cache_insert_futures).await;
        Ok(())
    }

    async fn load_libraries_impl(
        &self,
        hashes: &[TonHash],
    ) -> Result<Vec<ArcCell>, TonLibraryError> {
        let library_list: Vec<_> = hashes.iter().map(TonLibraryId::from).collect();
        let library_result = self
            .client
            .smc_get_libraries(&library_list)
            .await
            .map_err(|e| TonLibraryError::TonClientError(e.to_string()))?;

        let libraries: Vec<ArcCell> = library_result
            .result
            .into_iter()
            .map(|lib| BagOfCells::parse(&lib.data)?.single_root())
            .collect::<Result<_, _>>()?;

        Ok(libraries)
    }
}
