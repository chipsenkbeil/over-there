pub mod reply;
pub mod request;
mod transform;

pub use reply::{Reply, ReplyError};
pub use request::Request;
pub use transform::{
    LazilyTransformedContent, TransformContentError, TransformRule,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum Content {
    Request(Request),
    Reply(Reply),
}

impl Content {
    pub fn to_request(self) -> Option<Request> {
        match self {
            Self::Request(x) => Some(x),
            Self::Reply(_) => None,
        }
    }

    pub fn to_reply(self) -> Option<Reply> {
        match self {
            Self::Request(_) => None,
            Self::Reply(x) => Some(x),
        }
    }

    pub fn to_reply_error(self) -> Option<ReplyError> {
        match self.to_reply() {
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
