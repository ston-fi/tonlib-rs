use std::sync::Arc;
use std::time::Duration;

use lazy_static::lazy_static;

use crate::client::TonClientError;
use crate::tl::{TonFunction, TonNotification, TonResult};

/// The callback methods invoked by TonConnection
#[allow(unused_variables)]
pub trait TonConnectionCallback: Send + Sync {
    /// Method `on_invoke` gets called **before** invoking tonlib.
    fn on_invoke(&self, tag: &str, request_id: u32, function: &TonFunction) {}

    /// Method `on_invoke_result` gets called in two scenarios:
    ///
    /// - **after** receiving invoke result from tonlib and **before** sending result to the caller.
    /// - **after** failed attempt to invoke tonlib (this situation might occur only because of
    ///   serialization error).
    fn on_invoke_result(
        &self,
        tag: &str,
        request_id: u32,
        method: &str,
        duration: &Duration,
        result: &Result<TonResult, TonClientError>,
    ) {
    }

    /// Method `on_cancelled_invoke` gets called when attempt to send an invoke result is failed  
    ///
    /// Typically this happens when the corresponding future (async fn invoke_on_connection) is cancelled  
    fn on_cancelled_invoke(&self, tag: &str, request_id: u32, method: &str, duration: &Duration) {}

    /// Method `on_notification` gets called upon receiving valid notification from tonlib.
    ///
    /// A tonlib notification doesn't have corresponding request and thus no `request_id`.
    fn on_notification(&self, tag: &str, notification: &TonNotification) {}

    /// Method `on_ton_result_parse_error` gets called upon receiving message from tonlib
    /// that couldn't be parsed.
    ///
    /// Reception of `on_ton_result_parse_error` means that not all tonlib message get parsed
    /// and undefined behaviour is very likely.
    fn on_ton_result_parse_error(
        &self,
        tag: &str,
        request_extra: Option<&str>,
        result: &TonResult,
    ) {
    }

    /// Method `on_idle` gets called when polling tonlib returns `None`.
    fn on_idle(&self, tag: &str) {}

    /// Method `on_connection_loop_start` gets called when new connection loop starts
    fn on_connection_loop_start(&self, tag: &str) {}

    /// Method `on_connection_loop_exit` gets called when new connection loop stops and connection is dropped
    fn on_connection_loop_exit(&self, tag: &str) {}
}

/// An implementation of TonConnectionCallback that does nothing
pub struct NoopConnectionCallback {}

impl TonConnectionCallback for NoopConnectionCallback {}

/// An implementation of TonConnectionCallback that does default logging
pub struct LoggingConnectionCallback {}

impl TonConnectionCallback for LoggingConnectionCallback {
    fn on_invoke_result(
        &self,
        tag: &str,
        request_id: u32,
        method: &str,
        duration: &Duration,
        result: &Result<TonResult, TonClientError>,
    ) {
        match result {
            Ok(r) => {
                log::trace!(
                    "[{}] Invoke successful, request_id: {}, method: {}, elapsed: {:?}: {}",
                    tag,
                    request_id,
                    method,
                    duration,
                    r
                );
            }
            Err(e) => {
                log::warn!(
                    "[{}] Invocation error: request_id: {:?}, method: {}, elapsed: {:?}: {}",
                    tag,
                    request_id,
                    method,
                    duration,
                    e
                );
            }
        }
    }

    fn on_cancelled_invoke(&self, tag: &str, request_id: u32, method: &str, duration: &Duration) {
        log::warn!(
            "[{}] Error sending invoke result, receiver already closed. method: {} request_id: {}, elapsed: {:?}",
            tag,
            method,
            request_id,
            duration,
       );
    }

    fn on_notification(&self, tag: &str, notification: &TonNotification) {
        log::trace!("[{}] Sending notification: {:?}", tag, notification);
    }

    fn on_ton_result_parse_error(
        &self,
        tag: &str,
        request_extra: Option<&str>,
        result: &TonResult,
    ) {
        log::error!(
            "[{}] Error parsing result: request_extra: {:?}: {}",
            tag,
            request_extra,
            result
        );
    }

    fn on_connection_loop_start(&self, tag: &str) {
        log::info!("[{}] Starting event loop", tag);
    }

    fn on_connection_loop_exit(&self, tag: &str) {
        log::info!("[{}] Exiting event loop", tag);
    }
}

/// An implementation of TonConnectionCallback that invokes corresponding functions on
/// multiple child callbacks.
pub struct MultiConnectionCallback {
    callbacks: Vec<Arc<dyn TonConnectionCallback>>,
}

impl MultiConnectionCallback {
    pub fn new(callbacks: Vec<Arc<dyn TonConnectionCallback>>) -> MultiConnectionCallback {
        MultiConnectionCallback { callbacks }
    }
}

impl TonConnectionCallback for MultiConnectionCallback {
    fn on_invoke(&self, tag: &str, request_id: u32, function: &TonFunction) {
        for c in self.callbacks.iter() {
            c.on_invoke(tag, request_id, function)
        }
    }

    fn on_invoke_result(
        &self,
        tag: &str,
        request_id: u32,
        method: &str,
        duration: &Duration,
        res: &Result<TonResult, TonClientError>,
    ) {
        for c in self.callbacks.iter() {
            c.on_invoke_result(tag, request_id, method, duration, res)
        }
    }

    fn on_cancelled_invoke(&self, tag: &str, request_id: u32, method: &str, duration: &Duration) {
        for c in self.callbacks.iter() {
            c.on_cancelled_invoke(tag, request_id, method, duration)
        }
    }

    fn on_notification(&self, tag: &str, notification: &TonNotification) {
        for c in self.callbacks.iter() {
            c.on_notification(tag, notification)
        }
    }

    fn on_ton_result_parse_error(
        &self,
        tag: &str,
        request_extra: Option<&str>,
        result: &TonResult,
    ) {
        for c in self.callbacks.iter() {
            c.on_ton_result_parse_error(tag, request_extra, result)
        }
    }

    fn on_idle(&self, tag: &str) {
        for c in self.callbacks.iter() {
            c.on_idle(tag)
        }
    }

    fn on_connection_loop_start(&self, tag: &str) {
        for c in self.callbacks.iter() {
            c.on_connection_loop_start(tag)
        }
    }

    fn on_connection_loop_exit(&self, tag: &str) {
        for c in self.callbacks.iter() {
            c.on_connection_loop_exit(tag)
        }
    }
}

lazy_static! {
    pub static ref NOOP_CONNECTION_CALLBACK: Arc<dyn TonConnectionCallback + Send + Sync> =
        Arc::new(NoopConnectionCallback {});
    pub static ref LOGGING_CONNECTION_CALLBACK: Arc<dyn TonConnectionCallback + Send + Sync> =
        Arc::new(LoggingConnectionCallback {});
}
