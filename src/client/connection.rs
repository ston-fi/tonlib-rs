use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Weak};
use std::thread;
use std::time::Instant;

use anyhow::anyhow;
use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::{broadcast, oneshot};

use crate::client::{
    TonConnectionCallback, TonConnectionParams, TonError, TonFunctions, TonNotificationReceiver,
};
use crate::tl::stack::TvmStackEntry;
use crate::tl::types::{Config, KeyStoreType, Options, OptionsInfo, SmcMethodId, SmcRunResult};
use crate::tl::TlTonClient;
use crate::tl::TonFunction;
use crate::tl::TonNotification;
use crate::tl::TonResult;

struct RequestData {
    method: &'static str,
    send_time: Instant,
    sender: oneshot::Sender<anyhow::Result<TonResult>>,
}

type RequestMap = DashMap<u32, RequestData>;
type TonNotificationSender = broadcast::Sender<Arc<TonNotification>>;

struct Inner {
    tl_client: TlTonClient,
    counter: AtomicU32,
    request_map: RequestMap,
    notification_sender: TonNotificationSender,
    callback: Arc<dyn TonConnectionCallback + Send + Sync>,
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
    pub fn new(
        callback: Arc<dyn TonConnectionCallback + Send + Sync>,
    ) -> anyhow::Result<TonConnection> {
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
        callback: Arc<dyn TonConnectionCallback + Send + Sync>,
    ) -> anyhow::Result<TonConnection> {
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

    /// Attempts to initialize an existing TonConnection
    pub async fn init(
        &self,
        config: &str,
        blockchain_name: Option<&str>,
        use_callbacks_for_network: bool,
        ignore_cache: bool,
        keystore_type: KeyStoreType,
    ) -> anyhow::Result<OptionsInfo> {
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
            r => Err(anyhow!("Expected OptionsInfo, got: {:?}", r)),
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
    ) -> anyhow::Result<SmcRunResult> {
        let func = TonFunction::SmcRunGetMethod {
            id: id,
            method: method.clone(),
            stack: stack.to_vec(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::SmcRunResult(result) => Ok(result),
            r => Err(anyhow!("Expected SmcRunResult, got: {:?}", r)),
        }
    }
}

#[async_trait]
impl TonFunctions for TonConnection {
    async fn get_connection(&self) -> anyhow::Result<TonConnection> {
        Ok(self.clone())
    }

    async fn invoke_on_connection(
        &self,
        function: &TonFunction,
    ) -> anyhow::Result<(TonConnection, TonResult)> {
        let cnt = self.inner.counter.fetch_add(1, Ordering::SeqCst);
        let extra = cnt.to_string();
        let (tx, rx) = oneshot::channel::<anyhow::Result<TonResult>>();
        let data = RequestData {
            method: function.into(),
            send_time: Instant::now(),
            sender: tx,
        };
        self.inner.request_map.insert(cnt, data);
        self.inner.callback.on_invoke(cnt);
        let res = self.inner.tl_client.send(function, extra.as_str());
        if let Err(e) = res {
            let (_, data) = self.inner.request_map.remove(&cnt).unwrap();
            self.inner.callback.on_invoke_error(cnt, &e);
            data.sender.send(Err(e)).unwrap(); // Send should always succeed, so something went terribly wrong
        }
        let result = rx.await?;
        result.map(|r| (self.clone(), r))
    }
}

impl Clone for TonConnection {
    fn clone(&self) -> Self {
        let inner = self.inner.clone();
        TonConnection { inner }
    }
}

/// Client run loop
fn run_loop(tag: String, weak_inner: Weak<Inner>) -> anyhow::Result<()> {
    log::info!("[{}] Starting event loop", tag);
    loop {
        if let Some(inner) = weak_inner.upgrade() {
            let recv = inner.tl_client.receive(1.0);
            if let Some((ton_result, maybe_extra)) = recv {
                let maybe_request_id = maybe_extra.and_then(|s| s.parse::<u32>().ok());
                let maybe_data = maybe_request_id.and_then(|i| inner.request_map.remove(&i));
                let result: anyhow::Result<TonResult> =
                    if let Ok(TonResult::Error { code, message }) = ton_result {
                        inner
                            .callback
                            .on_tonlib_error(&maybe_request_id, code, &message);
                        let ton_error = TonError { code, message };
                        Err(anyhow::Error::from(ton_error))
                    } else {
                        ton_result
                    };

                match maybe_data {
                    Some((_, data)) => {
                        let request_id = maybe_request_id.unwrap(); // Can't be empty if data is not empty
                        let now = Instant::now();
                        let duration = now.duration_since(data.send_time);
                        inner.callback.on_invoke_result(
                            request_id,
                            data.method,
                            &duration,
                            &result,
                        );
                        log::debug!(
                            "[{}] Invoke successful, request_id: {}, method: {}, elapsed: {:?}",
                            tag,
                            request_id,
                            data.method,
                            &duration
                        );
                        if let Err(e) = data.sender.send(result) {
                            inner
                                .callback
                                .on_invoke_result_send_error(request_id, &duration, &e);
                            log::warn!(
                                "[{}] Error sending invoke result, method: {} request_id: {}: {:?}",
                                tag,
                                data.method,
                                request_id,
                                e
                            );
                        }
                    }
                    None => {
                        let maybe_notification =
                            result.and_then(|r| TonNotification::from_result(&r));
                        match maybe_notification {
                            Ok(notification) => {
                                inner.callback.on_notification(&notification);
                                if let Err(e) =
                                    inner.notification_sender.send(Arc::new(notification))
                                {
                                    log::warn!("[{}] Error sending notification: {}", tag, e);
                                }
                            }
                            Err(e) => {
                                inner.callback.on_notification_parse_error(&e);
                                log::warn!("[{}] Error parsing notification: {}", tag, e);
                            }
                        }
                    }
                }
                ()
            }
        } else {
            log::info!("[{}] Exiting event loop", tag);
            return Ok(());
        }
    }
}
