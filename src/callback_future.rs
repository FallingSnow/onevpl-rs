use std::cell::Cell;
// https://users.rust-lang.org/t/can-you-turn-a-callback-into-a-future-into-async-await/49378/16

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

#[derive(Clone)]
pub struct CbFuture<T>(Rc<CallbackFutureInner<T>>);

impl<T> CbFuture<T> {
    pub fn new() -> Self {
        Self(Rc::new(CallbackFutureInner::<T>::default()))
    }
}

struct CallbackFutureInner<T> {
    waker: Cell<Option<Waker>>,
    result: Cell<Option<T>>,
}

impl<T> Default for CallbackFutureInner<T> {
    fn default() -> Self {
        Self {
            waker: Cell::new(None),
            result: Cell::new(None),
        }
    }
}

impl<T> CbFuture<T> {
    // call this from your callback
    pub fn publish(&self, result: T) {
        self.0.result.set(Some(result));
        self.0.waker.take().map(|w| w.wake());
    }
}

impl<T> Future for CbFuture<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0.result.take() {
            Some(x) => Poll::Ready(x),
            None => {
                self.0.waker.set(Some(cx.waker().clone()));
                Poll::Pending
            }
        }
    }
}
