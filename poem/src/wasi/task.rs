use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    future::Future,
    rc::Rc,
};

use async_task::{Runnable, Task};

use crate::wasi::reactor::{poll, Events};

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

pub(crate) fn block_on<F: Future, T>(fut: F) -> F::Output
where
    F: Future<Output = T> + 'static,
    T: 'static,
{
    let mut events = Events::default();
    let res: Rc<Cell<Option<T>>> = Rc::new(Cell::new(None));

    spawn({
        let res = res.clone();
        async move { res.set(Some(fut.await)) }
    })
    .detach();

    loop {
        if let Some(res) = res.take() {
            return res;
        }

        QUEUE.with(|queue| loop {
            let item = queue.borrow_mut().pop_front();
            match item {
                Some(runnable) => {
                    runnable.run();
                }
                None => break,
            }
        });

        poll(&mut events);
    }
}
