use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Weak};
use std::thread;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, oneshot, Mutex, Semaphore, SemaphorePermit};

use crate::client::ext_data_provider::ExternalDataProvider;
use crate::client::{
    TonClientError, TonClientInterface, TonConnectionCallback, TonConnectionParams,
    TonNotificationReceiver,
};
use crate::tl::{
    BlockId, Config, KeyStoreType, Options, OptionsInfo, SmcRunResult, TlTonClient, TonFunction,
    TonNotification, TonResult, TonResultDiscriminants, TvmStackEntry,
};
use crate::types::TonMethodId;

pub const DEFAULT_NOTIFICATION_QUEUE_LENGTH: usize = 10000;
pub const DEFAULT_CONNECTION_CONCURRENCY_LIMIT: usize = 100;
pub const DEFAULT_UPDATE_INIT_BLOCK: bool = true;

struct RequestData {
    method: &'static str,
    send_time: Instant,
    sender: oneshot::Sender<Result<TonResult, TonClientError>>,
}

type RequestMap = Mutex<HashMap<u32, RequestData>>;
type TonNotificationSender = broadcast::Sender<Arc<TonNotification>>;

#[derive(Clone)]
pub struct TonConnection {
    inner: Arc<Inner>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionCheck {
    None,
    Health,  // ensure we connected to healthy node
    Archive, // ensure we connected to archive node
}

struct Inner {
    tl_client: TlTonClient,
    counter: AtomicU32,
    request_map: RequestMap,
    notification_sender: TonNotificationSender,
    callback: Arc<dyn TonConnectionCallback>,
    semaphore: Option<Semaphore>,
    external_data_provider: Option<Arc<dyn ExternalDataProvider>>,
}

static CONNECTION_COUNTER: AtomicU32 = AtomicU32::new(0);

impl TonConnection {
    pub async fn new(
        connection_check: ConnectionCheck,
        params: &TonConnectionParams,
        callback: Arc<dyn TonConnectionCallback>,
        external_data_provider: Option<Arc<dyn ExternalDataProvider>>,
    ) -> Result<TonConnection, TonClientError> {
        match connection_check {
            ConnectionCheck::None => new_connection(params, callback, external_data_provider).await,
            ConnectionCheck::Health => {
                new_connection_healthy(params, callback, external_data_provider).await
            }
            ConnectionCheck::Archive => {
                new_connection_archive(params, callback, external_data_provider).await
            }
        }
    }

    async fn init(&self, params: &TonConnectionParams) -> Result<OptionsInfo, TonClientError> {
        let keystore_type = match &params.keystore_dir {
            Some(keystore) => KeyStoreType::Directory {
                directory: keystore.clone(),
            },
            _ => KeyStoreType::InMemory,
        };

        let func = TonFunction::Init {
            options: Options {
                config: Config {
                    config: params.config.clone(),
                    blockchain_name: params.blockchain_name.clone(),
                    use_callbacks_for_network: params.use_callbacks_for_network,
                    ignore_cache: params.ignore_cache,
                },
                keystore_type,
            },
        };
        match self.invoke(&func).await? {
            TonResult::OptionsInfo(options_info) => Ok(options_info),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::OptionsInfo,
                r,
            )),
        }
    }

    pub fn subscribe(&self) -> TonNotificationReceiver {
        self.inner.notification_sender.subscribe()
    }

    pub async fn smc_run_get_method(
        &self,
        id: i64,
        method: &TonMethodId,
        stack: &[TvmStackEntry],
    ) -> Result<SmcRunResult, TonClientError> {
        let func = TonFunction::SmcRunGetMethod {
            id,
            method: method.into(),
            stack: stack.to_vec(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::SmcRunResult(result) => Ok(result),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::SmcRunResult,
                r,
            )),
        }
    }

    async fn limit_rate(&self) -> Result<Option<SemaphorePermit>, TonClientError> {
        match &self.inner.semaphore {
            Some(semaphore) => {
                let permit = semaphore.acquire().await.map_err(|_| {
                    TonClientError::InternalError("Failed to acquire semaphore permit".to_string())
                })?;
                Ok(Some(permit))
            }
            None => Ok(None),
        }
    }
}

async fn new_connection(
    params: &TonConnectionParams,
    callback: Arc<dyn TonConnectionCallback>,
    external_data_provider: Option<Arc<dyn ExternalDataProvider>>,
) -> Result<TonConnection, TonClientError> {
    let conn_id = CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tag = format!("ton-conn-{conn_id}");

    let (sender, _rcv) =
        broadcast::channel::<Arc<TonNotification>>(params.notification_queue_length);

    let semaphore = if params.concurrency_limit != 0 {
        Some(Semaphore::new(params.concurrency_limit))
    } else {
        None
    };

    let inner = Inner {
        tl_client: TlTonClient::new(tag.clone()),
        counter: AtomicU32::new(0),
        request_map: Mutex::new(HashMap::new()),
        notification_sender: sender,
        callback,
        semaphore,
        external_data_provider,
    };
    let inner_arc = Arc::new(inner);
    let inner_weak: Weak<Inner> = Arc::downgrade(&inner_arc);
    let thread_builder = thread::Builder::new().name(tag.clone());
    let callback = inner_arc.callback.clone();
    let _join_handle = thread_builder.spawn(|| run_loop(tag, inner_weak, callback))?;

    let conn = TonConnection { inner: inner_arc };
    let _info = conn.init(params).await?;

    Ok(conn)
}

async fn new_connection_healthy(
    params: &TonConnectionParams,
    callback: Arc<dyn TonConnectionCallback>,
    ext_data_provider: Option<Arc<dyn ExternalDataProvider>>,
) -> Result<TonConnection, TonClientError> {
    // connect to other node until it will be able to fetch the very first block
    loop {
        let conn = new_connection(params, callback.clone(), ext_data_provider.clone()).await?;
        let info_result = conn.get_masterchain_info().await;
        match info_result {
            Ok((_, info)) => {
                let block_result = conn.get_block_header(&info.last).await;
                if let Err(err) = block_result {
                    log::info!("Dropping connection to unhealthy node: {:?}", err);
                } else {
                    break Ok(conn);
                }
            }
            Err(err) => {
                log::info!("Dropping connection to unhealthy node: {:?}", err);
            }
        }
    }
}

async fn new_connection_archive(
    params: &TonConnectionParams,
    callback: Arc<dyn TonConnectionCallback>,
    ext_data_provider: Option<Arc<dyn ExternalDataProvider>>,
) -> Result<TonConnection, TonClientError> {
    // connect to other node until it will be able to fetch the very first block
    loop {
        let conn = new_connection(params, callback.clone(), ext_data_provider.clone()).await?;
        let info = BlockId {
            workchain: -1,
            shard: i64::MIN,
            seqno: 1,
        };
        conn.sync().await?;
        if conn.lookup_block(1, &info, 0, 0).await.is_ok() {
            return Ok(conn);
        }
        log::info!("Dropping connection to non-archive node, trying new one");
    }
}

#[async_trait]
impl TonClientInterface for TonConnection {
    async fn get_connection(&self) -> Result<TonConnection, TonClientError> {
        Ok(self.clone())
    }

    async fn invoke_on_connection(
        &self,
        function: &TonFunction,
    ) -> Result<(TonConnection, TonResult), TonClientError> {
        // TODO 2025.04.25 Sild it doesn't work because permit is dropped right after the call,
        // But it requires more investigation to understand if fix won't break anything else
        self.limit_rate().await?; // take the semaphore to limit number of simultaneous invokes being processed

        let cnt = self.inner.counter.fetch_add(1, Ordering::Relaxed);

        if let Some(external_provider) = &self.inner.external_data_provider {
            if let Some(result) = external_provider.handle(function).await {
                match result {
                    Ok(response) => return Ok((self.clone(), response)),
                    Err(err) => {
                        log::warn!("External data provider failed to handle function: {function:?} with err: {err:?}");
                    }
                }
            }
        }

        let extra = cnt.to_string();
        let (tx, rx) = oneshot::channel::<Result<TonResult, TonClientError>>();
        let data = RequestData {
            method: function.into(),
            send_time: Instant::now(),
            sender: tx,
        };
        self.inner.request_map.lock().await.insert(cnt, data);
        self.inner
            .callback
            .on_invoke(self.inner.tl_client.get_tag(), cnt, function);

        let res = self.inner.tl_client.send(function, extra.as_str());
        if let Err(e) = res {
            let data = self.inner.request_map.lock().await.remove(&cnt).unwrap();
            let tag = self.inner.tl_client.get_tag();
            let duration = data.send_time.elapsed();
            let res = Err(TonClientError::TlError(e));
            self.inner
                .callback
                .on_invoke_result(tag, cnt, data.method, &duration, &res);
            data.sender.send(res).unwrap(); // Send should always succeed, so something went terribly wrong
        }
        let maybe_result = rx.await;
        let result = match maybe_result {
            Ok(result) => result,
            Err(_) => {
                return Err(TonClientError::InternalError(
                    "Sender dropped without sending".to_string(),
                ));
            }
        };
        result.map(|r| (self.clone(), r))
    }
}

static NOT_AVAILABLE: &str = "N/A";

/// Client run loop
fn run_loop(tag: String, weak_inner: Weak<Inner>, callback: Arc<dyn TonConnectionCallback>) {
    callback.on_connection_loop_start(&tag);

    loop {
        if let Some(inner) = weak_inner.upgrade() {
            let recv = inner.tl_client.receive(1.0);
            if let Some((ton_result, maybe_extra)) = recv {
                let maybe_request_id = if let Some(s) = &maybe_extra {
                    s.parse::<u32>().ok()
                } else {
                    None
                };
                let maybe_data =
                    maybe_request_id.and_then(|i| inner.request_map.blocking_lock().remove(&i));
                let result: Result<TonResult, TonClientError> = match ton_result {
                    Ok(TonResult::Error { code, message }) => {
                        let method = maybe_data
                            .as_ref()
                            .map(|d| d.method)
                            .unwrap_or(NOT_AVAILABLE);
                        Err(TonClientError::TonlibError {
                            method,
                            code,
                            message,
                        })
                    }
                    Err(e) => Err(e.into()),
                    Ok(r) => Ok(r),
                };

                if let Some(data) = maybe_data {
                    // Found corresponding request, reply to it
                    let request_id = maybe_request_id.unwrap(); // Can't be empty if data is not empty
                    let now = Instant::now();
                    let duration = now.duration_since(data.send_time);
                    callback.on_invoke_result(&tag, request_id, data.method, &duration, &result);

                    if data.sender.send(result).is_err() {
                        callback.on_cancelled_invoke(&tag, request_id, data.method, &duration);
                    }
                } else {
                    // No request data, attempt to parse notification. Errors are ignored here.
                    if let Ok(r) = result {
                        let maybe_notification = TonNotification::from_result(&r);
                        if let Some(n) = maybe_notification {
                            callback.on_notification(&tag, &n);
                            // The call might only fail if there are no receivers, so just ignore the result
                            let _ = inner.notification_sender.send(Arc::new(n));
                        } else {
                            let extra = maybe_extra.as_deref();
                            callback.on_ton_result_parse_error(&tag, extra, &r);
                        }
                    }
                }
            } else {
                callback.on_idle(tag.as_str())
            }
        } else {
            callback.on_connection_loop_exit(tag.as_str());
            break;
        }
    }
}
