use std::{cell::RefCell, collections::VecDeque, future::Future};

use async_task::{Runnable, Task};

use crate::runtime::wasi::reactor::poll;

thread_local! {
    static QUEUE: RefCell<VecDeque<Runnable>> = RefCell::new(Default::default());
}

pub fn spawn<F, T>(fut: F) -> Task<T>
where
    F: Future<Output = T> + 'static,
    T: 'static,
{
    let schedule = |runnable| QUEUE.with(|queue| queue.borrow_mut().push_back(runnable));
    let (runnable, task) = async_task::spawn_local(fut, schedule);
    runnable.schedule();
    task
}

pub(crate) fn block_on<F: Future>(fut: F)
where
    F: Future + 'static,
{
    spawn(fut).detach();

    loop {
        QUEUE.with(|queue| loop {
            let item = queue.borrow_mut().pop_front();
            match item {
                Some(runnable) => {
                    runnable.run();
                }
                None => break,
            }
        });

        if !poll() {
            break;
        }
    }
}
