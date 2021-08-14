//! Thread pool for blocking operations

//! Because the thread pool does not implement Sync,
//! It cannot be initialized with `lazy_static`.
//! The Sync is implemented through Mutex wrapping.
//! To prevent subsequent use of the thread pool from blocking,
//! a thread pool handle is cloned via a thread local variable.

//! `poem` always provides the user first experience, come and use it ~

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use derive_more::Display;
use futures_channel::oneshot;
use parking_lot::Mutex;
use threadpool::ThreadPool;

/// Env variable for default cpu pool size.
const ENV_CPU_POOL_VAR: &str = "POEM_THREAD_POOL";

lazy_static::lazy_static! {
    pub(crate) static ref DEFAULT_POOL: Mutex<ThreadPool> = {

        let num = std::env::var(ENV_CPU_POOL_VAR)
        .map_err(|_| ())
        .and_then(|val| {
            val.parse().map_err(|_| log::warn!(
                "Can not parse {} value, using default",
                ENV_CPU_POOL_VAR,
            ))
        })
        .unwrap_or_else(|_| num_cpus::get() * 5);

        Mutex::new(threadpool::Builder::new()
        .thread_name("poem-web".to_owned())
        .num_threads(num)
        .build())
    };
}

thread_local! {
    static POOL: ThreadPool = {
        DEFAULT_POOL.lock().clone()
    };
}

/// Blocking operation execution error
#[derive(Debug, Display)]
pub enum BlockingError<E: fmt::Debug> {
    #[display(fmt = "{:?}", _0)]
    /// Arbitrary errors that occur during execution.
    Error(E),
    #[display(fmt = "Thread pool is gone")]
    /// Thread pool is gone.
    Canceled,
}

impl<E: fmt::Debug> std::error::Error for BlockingError<E> {}

/// Execute blocking function on a thread pool, returns future that resolves
/// to result of the function execution.
pub fn block_run<F, I, E>(f: F) -> CpuFuture<I, E>
where
    F: FnOnce() -> Result<I, E> + Send + 'static,
    I: Send + 'static,
    E: Send + fmt::Debug + 'static,
{
    let (tx, rx) = oneshot::channel();
    POOL.with(|pool| {
        pool.execute(move || {
            if !tx.is_canceled() {
                let _ = tx.send(f());
            }
        })
    });

    CpuFuture { rx }
}

/// Blocking operation completion future. It resolves with results
/// of blocking function execution.
pub struct CpuFuture<I, E> {
    rx: oneshot::Receiver<Result<I, E>>,
}

impl<I, E: fmt::Debug> Future for CpuFuture<I, E> {
    type Output = Result<I, BlockingError<E>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let rx = Pin::new(&mut self.rx);
        let res = match rx.poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(res) => res
                .map_err(|_| BlockingError::Canceled)
                .and_then(|res| res.map_err(BlockingError::Error)),
        };
        Poll::Ready(res)
    }
}
