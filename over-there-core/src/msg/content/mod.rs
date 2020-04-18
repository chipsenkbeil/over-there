pub mod reply;
pub mod request;

pub use reply::{Reply, ReplyError};
pub use request::{
    LazilyTransformedRequest, Request, TransformRequestError, TransformRule,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum Content {
    Request(Request),
    Reply(Reply),
}

impl Content {
    pub fn into_request(self) -> Option<Request> {
        match self {
            Self::Request(x) => Some(x),
            Self::Reply(_) => None,
        }
    }

    pub fn into_reply(self) -> Option<Reply> {
        match self {
            Self::Request(_) => None,
            Self::Reply(x) => Some(x),
        }
    }

    pub fn into_reply_error(self) -> Option<ReplyError> {
        match self.into_reply() {
            Some(Reply::Error(x)) => Some(x),
            _ => None,
        }
    }
}

impl From<Request> for Content {
    fn from(request: Request) -> Self {
        Self::Request(request)
    }
}

impl From<Reply> for Content {
    fn from(reply: Reply) -> Self {
        Self::Reply(reply)
    }
}

impl From<ReplyError> for Content {
    fn from(reply_error: ReplyError) -> Self {
        Self::from(Reply::Error(reply_error))
    }
}
