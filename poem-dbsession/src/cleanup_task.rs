#![allow(dead_code)]

use std::{future::Future, time::Duration};

use tokio::task::JoinHandle;

pub(crate) struct CleanupTask {
    handle: JoinHandle<()>,
}

impl Drop for CleanupTask {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl CleanupTask {
    pub(crate) fn new<F, Fut>(period: Duration, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(period).await;
                f().await;
            }
        });
        Self { handle }
    }
}
