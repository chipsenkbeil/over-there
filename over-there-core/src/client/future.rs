use crate::msg::Msg;
use over_there_derive::Error;
use over_there_utils::TtlValue;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

#[derive(Debug, Error)]
pub enum AskError {
    Failure { msg: String },
    InvalidResponse,
    Timeout,
}

pub(crate) struct AskFutureState {
    timer: TtlValue<()>,
    pub result: Option<Result<Msg, AskError>>,
    pub waker: Option<Waker>,
}

impl AskFutureState {
    pub fn new(ttl: Duration) -> Self {
        Self {
            timer: TtlValue::empty(ttl),
            result: None,
            waker: None,
        }
    }
}

pub struct AskFuture {
    pub(crate) state: Arc<Mutex<AskFutureState>>,
}

impl Future for AskFuture {
    type Output = Result<Msg, AskError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();

        if state.timer.has_expired() {
            Poll::Ready(Err(AskError::Timeout))
        } else if let Some(result) = state.result.take() {
            Poll::Ready(result)
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
