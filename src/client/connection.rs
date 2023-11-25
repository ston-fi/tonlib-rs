use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Weak};
use std::thread;
use std::time::Instant;

use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::{broadcast, oneshot};

use crate::tl::{
    Config, KeyStoreType, Options, OptionsInfo, SmcMethodId, SmcRunResult, TlTonClient,
    TonFunction, TonNotification, TonResult, TonResultDiscriminants, TvmStackEntry,
};
use crate::{
    client::{
        TonClientError, TonClientInterface, TonConnectionCallback, TonConnectionParams,
        TonNotificationReceiver,
    },
    tl::BlockId,
};

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
}

pub struct TonConnection {
    inner: Arc<Inner>,
}

static CONNECTION_COUNTER: AtomicU32 = AtomicU32::new(0);

impl TonConnection {
    /// Creates a new uninitialized TonConnection
    ///
    /// # Errors
    ///
    /// Returns error to capture any failure to create thread at system level
    pub fn new(callback: Arc<dyn TonConnectionCallback>) -> Result<TonConnection, TonClientError> {
        let tag = format!(
            "ton-conn-{}",
            CONNECTION_COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let (sender, receiver) = broadcast::channel::<Arc<TonNotification>>(10000); // TODO: Configurable
        let inner = Inner {
            tl_client: TlTonClient::new(tag.as_str()),
            counter: AtomicU32::new(0),
            request_map: RequestMap::new(),
            notification_sender: sender,
            callback,
            _notification_receiver: receiver,
        };
        let client = TonConnection {
            inner: Arc::new(inner),
        };
        let client_inner: Weak<Inner> = Arc::downgrade(&client.inner);
        let thread_builder = thread::Builder::new().name(tag.clone());
        thread_builder.spawn(|| run_loop(tag, client_inner))?;
        Ok(client)
    }

    /// Creates a new initialized TonConnection
    pub async fn connect(
        params: &TonConnectionParams,
        callback: Arc<dyn TonConnectionCallback>,
    ) -> Result<TonConnection, TonClientError> {
        let conn = Self::new(callback)?;
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
        Ok(conn)
    }

    pub async fn connect_to_archive(
        params: &TonConnectionParams,
        callback: Arc<dyn TonConnectionCallback>,
    ) -> Result<TonConnection, TonClientError> {
        // connect to other node until it will be able to fetch the very first block
        loop {
            let c = TonConnection::connect(params, callback.clone()).await?;
            let info = BlockId {
                workchain: -1,
                shard: i64::MIN,
                seqno: 1,
            };
            let r = c.lookup_block(1, &info, 0, 0).await;
            if r.is_ok() {
                break Ok(c);
            } else {
                log::info!("Dropping connection to non-archive node");
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
                    blockchain_name: blockchain_name.map(|s| String::from(s)),
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
                TonResultDiscriminants::OptionsInfo.into(),
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
        method: &SmcMethodId,
        stack: &Vec<TvmStackEntry>,
    ) -> Result<SmcRunResult, TonClientError> {
        let func = TonFunction::SmcRunGetMethod {
            id: id,
            method: method.clone(),
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
                return Err(TonClientError::InternalError {
                    message: "Sender dropped without sending".to_string(),
                });
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
fn run_loop(tag: String, weak_inner: Weak<Inner>) {
    log::info!("[{}] Starting event loop", tag);
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
                    inner.callback.on_invoke_result(
                        &tag,
                        request_id,
                        data.method,
                        &duration,
                        &result,
                    );

                    if let Err(_) = data.sender.send(result) {
                        log::warn!(
                                "[{}] Error sending invoke result, receiver already closed. method: {} request_id: {}, elapsed: {:?}",
                                tag,
                                data.method,
                                request_id,
                                &duration,
                            );
                    }
                } else {
                    // No request data, attempt to parse notification. Errors are ignored here.
                    if let Ok(r) = result {
                        let maybe_notification = TonNotification::from_result(&r);
                        if let Some(n) = maybe_notification {
                            inner.callback.on_notification(&tag, &n);
                            // The call might only fail if there are no receivers, so just ignore the result
                            let _ = inner.notification_sender.send(Arc::new(n));
                        } else {
                            let extra = match &maybe_extra {
                                Some(s) => Some(s.as_str()),
                                None => None,
                            };
                            inner.callback.on_ton_result_parse_error(&tag, extra, &r);
                        }
                    }
                }
            }
        } else {
            log::info!("[{}] Exiting event loop", tag);
            break;
        }
    }
}
