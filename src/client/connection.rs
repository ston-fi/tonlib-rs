use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Weak};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::{broadcast, oneshot, Semaphore, SemaphorePermit};

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

struct RequestData {
    method: &'static str,
    send_time: Instant,
    sender: oneshot::Sender<Result<TonResult, TonClientError>>,
}

type RequestMap = DashMap<u32, RequestData>;
type TonNotificationSender = broadcast::Sender<Arc<TonNotification>>;

struct Inner {
    tl_client: TlTonClient,
    counter: AtomicU32,
    request_map: RequestMap,
    notification_sender: TonNotificationSender,
    callback: Arc<dyn TonConnectionCallback>,
    _notification_receiver: TonNotificationReceiver,
    semaphore: Option<Semaphore>,
}

pub struct TonConnection {
    inner: Arc<Inner>,
}

static CONNECTION_COUNTER: AtomicU32 = AtomicU32::new(0);

impl TonConnection {
    /// Creates a new uninitialized TonConnection.
    ///
    /// # Errors
    ///
    /// Returns error to capture any failure to create thread at system level
    pub fn new(
        callback: Arc<dyn TonConnectionCallback>,
        params: &TonConnectionParams,
    ) -> Result<TonConnection, TonClientError> {
        Self::new_joinable(callback, params).map(|r| r.0)
    }

    /// Creates a new uninitialized TonConnection together with its `JoinHandle`.
    ///
    /// # Errors
    ///
    /// Returns error to capture any failure to create thread at system level
    pub(crate) fn new_joinable(
        callback: Arc<dyn TonConnectionCallback>,
        params: &TonConnectionParams,
    ) -> Result<(TonConnection, JoinHandle<()>), TonClientError> {
        let tag = format!(
            "ton-conn-{}",
            CONNECTION_COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let (sender, receiver) =
            broadcast::channel::<Arc<TonNotification>>(params.notification_queue_length);
        let concurrency_limit = params.concurrency_limit;
        let semaphore = if concurrency_limit != 0 {
            Some(Semaphore::new(params.concurrency_limit))
        } else {
            None
        };
        let inner = Inner {
            tl_client: TlTonClient::new(tag.as_str()),
            counter: AtomicU32::new(0),
            request_map: RequestMap::new(),
            notification_sender: sender,
            callback,
            _notification_receiver: receiver,
            semaphore,
        };
        let inner_arc = Arc::new(inner);
        let inner_weak: Weak<Inner> = Arc::downgrade(&inner_arc);
        let thread_builder = thread::Builder::new().name(tag.clone());
        let callback = inner_arc.callback.clone();
        let join_handle = thread_builder.spawn(|| run_loop(tag, inner_weak, callback))?;
        let conn = TonConnection { inner: inner_arc };
        Ok((conn, join_handle))
    }

    /// Creates a new initialized TonConnection
    pub async fn connect(
        params: &TonConnectionParams,
        callback: Arc<dyn TonConnectionCallback>,
    ) -> Result<TonConnection, TonClientError> {
        Self::connect_joinable(params, callback).await.map(|r| r.0)
    }

    /// Creates a new initialized TonConnection
    pub async fn connect_joinable(
        params: &TonConnectionParams,
        callback: Arc<dyn TonConnectionCallback>,
    ) -> Result<(TonConnection, JoinHandle<()>), TonClientError> {
        let (conn, join_handle) = Self::new_joinable(callback, params)?;
        let keystore_type = if let Some(directory) = &params.keystore_dir {
            KeyStoreType::Directory {
                directory: directory.clone(),
            }
        } else {
            KeyStoreType::InMemory
        };
        let _ = conn
            .init(
                params.config.as_str(),
                params.blockchain_name.as_deref(),
                params.use_callbacks_for_network,
                params.ignore_cache,
                keystore_type,
            )
            .await?;
        Ok((conn, join_handle))
    }

    pub(crate) async fn connect_archive(
        params: &TonConnectionParams,
        callback: Arc<dyn TonConnectionCallback>,
    ) -> Result<(TonConnection, JoinHandle<()>), TonClientError> {
        // connect to other node until it will be able to fetch the very first block
        loop {
            let (conn, join_handle) = Self::connect_joinable(params, callback.clone()).await?;
            let info = BlockId {
                workchain: -1,
                shard: i64::MIN,
                seqno: 1,
            };
            let r = conn.lookup_block(1, &info, 0, 0).await;
            if r.is_ok() {
                break Ok((conn, join_handle));
            } else {
                log::info!("Dropping connection to non-archive node");
            }
        }
    }

    pub(crate) async fn connect_healthy(
        params: &TonConnectionParams,
        callback: Arc<dyn TonConnectionCallback>,
    ) -> Result<(TonConnection, JoinHandle<()>), TonClientError> {
        // connect to other node until it will be able to fetch the very first block
        loop {
            let (conn, join_handle) =
                TonConnection::connect_joinable(params, callback.clone()).await?;
            let info_result = conn.get_masterchain_info().await;
            match info_result {
                Ok((_, info)) => {
                    let block_result = conn.get_block_header(&info.last).await;
                    if let Err(err) = block_result {
                        log::info!("Dropping connection to unhealthy node: {:?}", err);
                    } else {
                        break Ok((conn, join_handle));
                    }
                }
                Err(err) => {
                    log::info!("Dropping connection to unhealthy node: {:?}", err);
                }
            }
        }
    }

    /// Attempts to initialize an existing TonConnection
    pub async fn init(
        &self,
        config: &str,
        blockchain_name: Option<&str>,
        use_callbacks_for_network: bool,
        ignore_cache: bool,
        keystore_type: KeyStoreType,
    ) -> Result<OptionsInfo, TonClientError> {
        let func = TonFunction::Init {
            options: Options {
                config: Config {
                    config: String::from(config),
                    blockchain_name: blockchain_name.map(String::from),
                    use_callbacks_for_network,
                    ignore_cache,
                },
                keystore_type,
            },
        };
        let result = self.invoke(&func).await?;
        match result {
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
        Ok(if let Some(semaphore) = &self.inner.semaphore {
            Some(
                semaphore
                    .acquire()
                    .await
                    .map_err(|_| TonClientError::InternalError("AcquireError".to_string()))?,
            )
        } else {
            None
        })
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
        self.limit_rate().await?; // take the semaphore to limit number of simultaneous invokes being processed
        let cnt = self.inner.counter.fetch_add(1, Ordering::SeqCst);
        let extra = cnt.to_string();
        let (tx, rx) = oneshot::channel::<Result<TonResult, TonClientError>>();
        let data = RequestData {
            method: function.into(),
            send_time: Instant::now(),
            sender: tx,
        };
        self.inner.request_map.insert(cnt, data);
        self.inner
            .callback
            .on_invoke(self.inner.tl_client.get_tag(), cnt, function);

        let res = self.inner.tl_client.send(function, extra.as_str());
        if let Err(e) = res {
            let (_, data) = self.inner.request_map.remove(&cnt).unwrap();
            let tag = self.inner.tl_client.get_tag();
            let duration = Instant::now().duration_since(data.send_time);
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

impl Clone for TonConnection {
    fn clone(&self) -> Self {
        let inner = self.inner.clone();
        TonConnection { inner }
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
                let maybe_data = maybe_request_id.and_then(|i| inner.request_map.remove(&i));
                let result: Result<TonResult, TonClientError> = match ton_result {
                    Ok(TonResult::Error { code, message }) => {
                        let method = maybe_data
                            .as_ref()
                            .map(|d| d.1.method)
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

                if let Some((_, data)) = maybe_data {
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
