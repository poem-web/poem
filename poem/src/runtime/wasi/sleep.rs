use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures_util::Stream;
use libwasi::CLOCKID_MONOTONIC;
use poem_wasm::Subscription;

use crate::runtime::wasi::reactor;

pub struct Sleep {
    registered: bool,
    deadline: u64,
}

#[inline]
fn get_time() -> u64 {
    unsafe { libwasi::clock_time_get(CLOCKID_MONOTONIC, 0).unwrap() / 1000000 }
}

#[inline]
pub fn sleep(delay: Duration) -> Sleep {
    Sleep {
        registered: false,
        deadline: get_time() + delay.as_millis() as u64,
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;

        if !this.registered {
            this.registered = true;
            reactor::register(Subscription::timeout(this.deadline), cx);
            return Poll::Pending;
        } else {
            if get_time() >= this.deadline {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
}

pub struct IntervalStream {
    registered: bool,
    period: Duration,
    deadline: u64,
}

impl IntervalStream {
    pub fn new(period: Duration) -> Self {
        Self {
            registered: false,
            period,
            deadline: get_time() + period.as_millis() as u64,
        }
    }
}

impl Stream for IntervalStream {
    type Item = Instant;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        if !this.registered {
            this.registered = true;
            reactor::register(Subscription::timeout(this.deadline), cx);
            return Poll::Pending;
        } else {
            if get_time() >= this.deadline {
                this.deadline = this.deadline + this.period.as_millis() as u64;
                reactor::register(Subscription::timeout(this.deadline), cx);
                Poll::Ready(Some(Instant::now()))
            } else {
                Poll::Pending
            }
        }
    }
}
