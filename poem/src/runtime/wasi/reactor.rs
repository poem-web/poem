use std::{
    cell::RefCell,
    mem::MaybeUninit,
    task::{Context, Waker},
};

use poem_wasm::{ffi, Event, Subscription};

thread_local! {
    static REACTOR: RefCell<Reactor> = RefCell::new(Reactor::new());
}

struct Reactor {
    subscriptions: Vec<Subscription>,
    wakers: Vec<Waker>,
    id: u64,
}

impl Reactor {
    fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
            wakers: Vec::new(),
            id: 0,
        }
    }
}

pub(crate) fn register(subscription: Subscription, cx: &Context<'_>) {
    REACTOR.with(|reactor| {
        let mut reactor = reactor.borrow_mut();
        let waker = cx.waker().clone();
        let id = reactor.id;
        reactor.id += 1;

        reactor.subscriptions.push(subscription.userdata(id));
        reactor.wakers.push(waker);
    });
}

pub(crate) fn poll() -> bool {
    REACTOR.with(|reactor| {
        let mut reactor = reactor.borrow_mut();

        unsafe {
            let num_subscriptions = reactor.subscriptions.len();
            let mut event: MaybeUninit<Event> = MaybeUninit::uninit();

            if num_subscriptions == 0 {
                return false;
            }

            ffi::poll(
                reactor.subscriptions.as_ptr() as u32,
                num_subscriptions as u32,
                event.as_mut_ptr() as u32,
            );

            let event = event.assume_init();
            if let Some(idx) = reactor
                .subscriptions
                .iter()
                .position(|subscription| subscription.get_userdata() == event.userdata())
            {
                reactor.subscriptions.swap_remove(idx);
                reactor.wakers.swap_remove(idx).wake();
            }

            true
        }
    })
}
