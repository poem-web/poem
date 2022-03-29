use std::{
    cell::RefCell,
    mem::MaybeUninit,
    task::{Context, Waker},
};

use libwasi::Subscription;

thread_local! {
    static REACTOR: RefCell<Reactor> = RefCell::new(Reactor::new());
}

#[derive(Default)]
pub(crate) struct Events(Vec<MaybeUninit<libwasi::Event>>);

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

pub(crate) fn register_timeout(deadline: libwasi::Timestamp, cx: &Context<'_>) {
    REACTOR.with(|reactor| {
        let mut reactor = reactor.borrow_mut();
        let waker = cx.waker().clone();
        let id = reactor.id;
        reactor.id += 1;

        reactor.subscriptions.push(libwasi::Subscription {
            userdata: id,
            u: libwasi::SubscriptionU {
                tag: libwasi::EVENTTYPE_CLOCK.raw(),
                u: libwasi::SubscriptionUU {
                    clock: libwasi::SubscriptionClock {
                        id: libwasi::CLOCKID_MONOTONIC,
                        timeout: deadline,
                        precision: 0,
                        flags: libwasi::SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME,
                    },
                },
            },
        });
        reactor.wakers.push(waker);
    });
}

pub(crate) fn register_read(fd: libwasi::Fd, cx: &Context<'_>) {
    REACTOR.with(|reactor| {
        let mut reactor = reactor.borrow_mut();
        let waker = cx.waker().clone();
        let id = reactor.id;
        reactor.id += 1;

        reactor.subscriptions.push(libwasi::Subscription {
            userdata: id,
            u: libwasi::SubscriptionU {
                tag: libwasi::EVENTTYPE_FD_READ.raw(),
                u: libwasi::SubscriptionUU {
                    fd_read: libwasi::SubscriptionFdReadwrite {
                        file_descriptor: fd,
                    },
                },
            },
        });
        reactor.wakers.push(waker);
    });
}

pub(crate) fn register_write(fd: libwasi::Fd, cx: &Context<'_>) {
    REACTOR.with(|reactor| {
        let mut reactor = reactor.borrow_mut();
        let waker = cx.waker().clone();
        let id = reactor.id;
        reactor.id += 1;

        reactor.subscriptions.push(libwasi::Subscription {
            userdata: id,
            u: libwasi::SubscriptionU {
                tag: libwasi::EVENTTYPE_FD_WRITE.raw(),
                u: libwasi::SubscriptionUU {
                    fd_read: libwasi::SubscriptionFdReadwrite {
                        file_descriptor: fd,
                    },
                },
            },
        });
        reactor.wakers.push(waker);
    });
}

pub(crate) fn poll(events: &mut Events) -> bool {
    REACTOR.with(|reactor| {
        let mut reactor = reactor.borrow_mut();

        if reactor.subscriptions.is_empty() {
            return false;
        }

        unsafe {
            let num_subscriptions = reactor.subscriptions.len();
            events.0.reserve(num_subscriptions);

            let num_events = libwasi::poll_oneoff(
                reactor.subscriptions.as_ptr(),
                events.0.as_mut_ptr().cast(),
                num_subscriptions,
            )
            .unwrap();
            events.0.set_len(num_events);

            for event in &events.0 {
                let event = event.assume_init_ref();
                if let Some(idx) = reactor
                    .subscriptions
                    .iter()
                    .position(|subscription| subscription.userdata == event.userdata)
                {
                    reactor.subscriptions.swap_remove(idx);
                    reactor.wakers.swap_remove(idx).wake();
                }
            }
        }

        true
    })
}
