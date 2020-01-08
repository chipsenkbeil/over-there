pub mod content;

use chrono::prelude::{DateTime, Utc};
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Msg {
    /// ID associated with a request or response
    pub id: u32,

    /// The time at which the message was created
    pub creation_date: DateTime<Utc>,

    /// Content within the message
    pub content: content::Content,
}

impl Msg {
    pub fn to_vec(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(&self)
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read_ref(slice)
    }
}
//
// GENERAL CONTENT CONVERSION
//

impl From<content::Content> for Msg {
    fn from(content: content::Content) -> Self {
        Self {
            id: random(),
            creation_date: Utc::now(),
            content,
        }
    }
}

//
// GENERAL REQUEST/RESPONSE CONVERSIONS
//

impl From<content::request::Request> for Msg {
    fn from(request: content::request::Request) -> Self {
        Self::from(content::Content::from(request))
    }
}

impl From<content::response::Response> for Msg {
    fn from(response: content::response::Response) -> Self {
        Self::from(content::Content::from(response))
    }
}

//
// SPECIFIC REQUEST CONVERSIONS
//

impl From<content::request::standard::StandardRequest> for Msg {
    fn from(r: content::request::standard::StandardRequest) -> Self {
        Self::from(content::Content::from(content::request::Request::from(r)))
    }
}

#[cfg(feature = "custom")]
impl From<content::request::custom::CustomRequest> for Msg {
    fn from(r: content::request::custom::CustomRequest) -> Self {
        Self::from(content::Content::from(content::request::Request::from(r)))
    }
}

#[cfg(feature = "exec")]
impl From<content::request::exec::ExecRequest> for Msg {
    fn from(r: content::request::exec::ExecRequest) -> Self {
        Self::from(content::Content::from(content::request::Request::from(r)))
    }
}

#[cfg(feature = "forward")]
impl From<content::request::forward::ForwardRequest> for Msg {
    fn from(r: content::request::forward::ForwardRequest) -> Self {
        Self::from(content::Content::from(content::request::Request::from(r)))
    }
}

#[cfg(feature = "file-system")]
impl From<content::request::file_system::FileSystemRequest> for Msg {
    fn from(r: content::request::file_system::FileSystemRequest) -> Self {
        Self::from(content::Content::from(content::request::Request::from(r)))
    }
}

//
// SPECIFIC RESPONSE CONVERSIONS
//

impl From<content::response::standard::StandardResponse> for Msg {
    fn from(r: content::response::standard::StandardResponse) -> Self {
        Self::from(content::Content::from(content::response::Response::from(r)))
    }
}

#[cfg(feature = "custom")]
impl From<content::response::custom::CustomResponse> for Msg {
    fn from(r: content::response::custom::CustomResponse) -> Self {
        Self::from(content::Content::from(content::response::Response::from(r)))
    }
}

#[cfg(feature = "exec")]
impl From<content::response::exec::ExecResponse> for Msg {
    fn from(r: content::response::exec::ExecResponse) -> Self {
        Self::from(content::Content::from(content::response::Response::from(r)))
    }
}

#[cfg(feature = "forward")]
impl From<content::response::forward::ForwardResponse> for Msg {
    fn from(r: content::response::forward::ForwardResponse) -> Self {
        Self::from(content::Content::from(content::response::Response::from(r)))
    }
}

#[cfg(feature = "file-system")]
impl From<content::response::file_system::FileSystemResponse> for Msg {
    fn from(r: content::response::file_system::FileSystemResponse) -> Self {
        Self::from(content::Content::from(content::response::Response::from(r)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_should_produce_a_new_msg_with_that_content() {
        let msg = Msg::from(content::request::standard::StandardRequest::HeartbeatRequest);

        // Verify creation date was set to around now
        assert!(
            Utc::now()
                .signed_duration_since(msg.creation_date)
                .num_milliseconds()
                >= 0,
            "Unexpected creation date: {:?}",
            msg.creation_date
        );

        // Verify that our message was set to the right type
        match msg.content {
            content::Content::Request(content::request::Request::Standard(
                content::request::standard::StandardRequest::HeartbeatRequest,
            )) => (),
            x => panic!("Unexpected content: {:?}", x),
        }
    }
}
