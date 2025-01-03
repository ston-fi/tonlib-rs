use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use moka::future::Cache;
use tonlib_core::cell::dict::predefined_writers::val_writer_ref_cell;
use tonlib_core::cell::{ArcCell, BagOfCells, CellBuilder};
use tonlib_core::TonHash;

use super::{ContractLibraryDict, LibraryLoader};
use crate::contract::TonLibraryError;

const DEFAULT_LIBRARY_CACHE_CAPACITY: u64 = 300;
const DEFAULT_LIBRARY_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(60 * 60);
const DEFAULT_INTERNAL_CACHE_CAPACITY: u64 = 300;
const DEFAULT_INTERNAL_CACHE_TIME_TO_LIVE: Duration = Duration::from_secs(60 * 60);

const DEFAULT_LIBRARY_CACHE_PARAMS: LibraryCacheParams = LibraryCacheParams {
    lib_cache_capacity: DEFAULT_LIBRARY_CACHE_CAPACITY,
    lib_cache_time_to_live: DEFAULT_LIBRARY_CACHE_TIME_TO_LIVE,
    internal_cache_capacity: DEFAULT_INTERNAL_CACHE_CAPACITY,
    internal_cache_time_to_live: DEFAULT_INTERNAL_CACHE_TIME_TO_LIVE,
};

#[derive(Clone, Copy)]
pub struct LibraryCacheParams {
    lib_cache_capacity: u64,
    lib_cache_time_to_live: Duration,
    internal_cache_capacity: u64,
    internal_cache_time_to_live: Duration,
}

#[derive(Clone)]
pub struct LibraryProvider {
    inner: Arc<Inner>,
}

struct Inner {
    loader: Arc<dyn LibraryLoader>,
    cache_by_hash_seqno: Cache<(TonHash, i32), Option<ArcCell>>,
    cache_by_hash: Cache<TonHash, ArcCell>,
    current_seqno: Arc<AtomicI32>,
}

impl LibraryProvider {
    pub fn new(
        loader: Arc<dyn LibraryLoader>,
        cache_params: Option<LibraryCacheParams>,
        current_seqno: Arc<AtomicI32>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner::new(loader, cache_params, current_seqno)),
        }
    }

    pub fn extract_library_hashes<'a, I>(cells: I) -> Result<Vec<TonHash>, TonLibraryError>
    where
        I: IntoIterator<Item = &'a ArcCell>,
    {
        let mut arc_cell_buffer = VecDeque::new();

        arc_cell_buffer.extend(cells);

        let mut lib_cells = HashSet::new();
        let mut visited_cells = HashSet::new();

        while let Some(cell) = arc_cell_buffer.pop_front() {
            if !visited_cells.insert(cell) {
                continue;
            }
            let refs = cell.references();
            arc_cell_buffer.extend(refs.iter());
            if cell.is_library() {
                lib_cells.insert(cell.clone());
            }
        }

        let lib_hashes: Vec<TonHash> = lib_cells
            .iter()
            .map(|i| i.data()[1..].try_into())
            .collect::<Result<_, _>>()?;

        Ok(lib_hashes)
    }

    pub async fn get_libs_by_seqno<'a, I>(
        &self,
        cells: I,
        seqno: i32,
    ) -> Result<ContractLibraryDict, TonLibraryError>
    where
        I: IntoIterator<Item = &'a ArcCell>,
    {
        let lib_hashes = LibraryProvider::extract_library_hashes(cells)?;
        let libs = self.inner.get_libs(lib_hashes.as_slice(), seqno).await?;
        LibraryProvider::store_to_dict(lib_hashes.as_slice(), libs)
    }

    pub async fn get_libs_latest<'a, I>(
        &self,
        cells: I,
    ) -> Result<ContractLibraryDict, TonLibraryError>
    where
        I: IntoIterator<Item = &'a ArcCell>,
    {
        let seqno = self.inner.current_seqno.load(Ordering::Relaxed);
        self.get_libs_by_seqno(cells, seqno).await
    }

    fn store_to_dict(
        library_hashes: &[TonHash],
        lib_hashmap: HashMap<TonHash, ArcCell>,
    ) -> Result<ContractLibraryDict, TonLibraryError> {
        let lib_cell = CellBuilder::new()
            .store_dict_data(256, val_writer_ref_cell, lib_hashmap)?
            .build()?;

        let dict_boc = BagOfCells::from_root(lib_cell).serialize(false)?;
        let keys = library_hashes.to_vec();
        let dict = ContractLibraryDict { dict_boc, keys };
        Ok(dict)
    }
}

impl Inner {
    pub(crate) fn new(
        loader: Arc<dyn LibraryLoader>,
        cache_params: Option<LibraryCacheParams>,
        current_seqno: Arc<AtomicI32>,
    ) -> Self {
        let cache_params = cache_params.unwrap_or(DEFAULT_LIBRARY_CACHE_PARAMS);

        let library_cache = Cache::builder()
            .max_capacity(cache_params.lib_cache_capacity)
            .time_to_live(cache_params.lib_cache_time_to_live)
            .build();
        let internal_cache = Cache::builder()
            .max_capacity(cache_params.internal_cache_capacity)
            .time_to_live(cache_params.internal_cache_time_to_live)
            .build();
        Self {
            loader,
            cache_by_hash_seqno: library_cache,
            cache_by_hash: internal_cache,
            current_seqno,
        }
    }

    pub(crate) async fn get_libs(
        &self,
        lib_hashes: &[TonHash],
        seqno: i32,
    ) -> Result<HashMap<TonHash, ArcCell>, TonLibraryError>
where {
        let mut result_libs = HashMap::new();

        let keys = lib_hashes
            .iter()
            .map(|h| (*h, seqno))
            .collect::<HashSet<_>>();
        let cached_libs_future = keys
            .iter()
            .map(|key| async move { (key.0, self.cache_by_hash_seqno.get(key).await) });
        let maybe_cached_libs = join_all(cached_libs_future).await;

        let mut hashes_to_load = vec![];
        for (hash, value) in maybe_cached_libs {
            // outer option means whether library is in the cache or not
            // inner option means whether library is available at certain seqno
            match value {
                Some(Some(lib)) => {
                    result_libs.insert(hash, lib);
                }
                Some(None) => {}
                None => {
                    log::trace!("loading lib from BC: {:?}", hash);
                    hashes_to_load.push(hash);
                }
            }
        }

        // load libs from blockchain
        let mut blockchain_libs = self
            .loader
            .load_libraries(hashes_to_load.as_slice(), Some(seqno))
            .await?;
        self.replace_by_existing_data(&mut blockchain_libs).await;
        result_libs.extend(blockchain_libs.iter().map(|l| (l.cell_hash(), l.clone())));

        self.insert_to_lib_cache(blockchain_libs, seqno).await?;
        Ok(result_libs)
    }

    async fn insert_to_lib_cache(
        &self,
        libs: Vec<ArcCell>,
        seqno: i32,
    ) -> Result<(), TonLibraryError> {
        let mut cache_insert_futures = Vec::with_capacity(libs.len());
        let mut internal_cache_insert_futures = Vec::with_capacity(libs.len());
        for lib in libs {
            cache_insert_futures.push(
                self.cache_by_hash_seqno
                    .insert((lib.cell_hash(), seqno), Some(lib.clone())),
            );
            internal_cache_insert_futures.push(self.cache_by_hash.insert(lib.cell_hash(), lib));
        }

        join_all(cache_insert_futures).await;
        join_all(internal_cache_insert_futures).await;
        Ok(())
    }

    async fn replace_by_existing_data(&self, loaded_libs: &mut [ArcCell]) {
        let loaded_libs_keys: Vec<_> = loaded_libs.iter().map(|l| l.cell_hash()).collect();
        let future_internals = loaded_libs_keys.iter().map(|k| self.cache_by_hash.get(k));

        let internal_cached_libs = join_all(future_internals).await;

        // replace loaded libs from blockchain to ones from internal cache.
        for i in 0..loaded_libs.len() {
            if let Some(lib) = &internal_cached_libs[i] {
                loaded_libs[i] = lib.clone()
            }
        }
    }
}
