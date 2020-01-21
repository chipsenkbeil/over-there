mod heartbeat;
mod unknown;

use crate::msg::{content::ContentType, Msg, MsgError};
use over_there_transport::NetSend;
use std::sync::mpsc::SendError;

pub type ActionFP<NS: NetSend> = fn(Msg, NS) -> Result<(), ActionError<NS::TSendData>>;

pub enum ActionError<T> {
    MsgError(MsgError),
    SendError(SendError<T>),
    Unknown,
}

impl<T> std::fmt::Debug for ActionError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MsgError(e) => write!(f, "{:?}", e),
            Self::SendError(e) => write!(f, "{:?}", e),
            Self::Unknown => write!(f, "Unknown error"),
        }
    }
}

impl<T> std::fmt::Display for ActionError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MsgError(e) => write!(f, "{:?}", e),
            Self::SendError(e) => write!(f, "{:?}", e),
            Self::Unknown => write!(f, "Unknown error"),
        }
    }
}

impl<T> std::error::Error for ActionError<T> {}

/// Looks up an appropriate function pointer for the given content type
pub fn route<NS: NetSend>(content_type: ContentType) -> ActionFP<NS> {
    match content_type {
        ContentType::HeartbeatRequest => heartbeat::heartbeat_request,
        ContentType::HeartbeatResponse => heartbeat::heartbeat_response,

        // TODO: Remove unknown by completing all other content types
        _ => unknown::unknown,
    }
}

/// Evaluate a message's content and potentially respond using the provided
/// netsend component
pub fn execute<NS: NetSend>(msg: Msg, ns: NS) -> Result<(), ActionError<NS::TSendData>> {
    (route(ContentType::from(&msg.content)))(msg, ns)
}
