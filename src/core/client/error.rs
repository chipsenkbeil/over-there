use crate::core::Reply;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io;

#[derive(Serialize, Deserialize, Debug, Display, PartialEq, Eq)]
pub enum SendError {
    EncodingFailed,
    SendFailed,
}

impl Error for SendError {}

impl From<AskError> for Option<SendError> {
    fn from(error: AskError) -> Self {
        match error {
            AskError::EncodingFailed => Some(SendError::EncodingFailed),
            AskError::SendFailed => Some(SendError::SendFailed),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Display, PartialEq, Eq)]
pub enum AskError {
    #[display(fmt = "Failed: {}", msg)]
    Failure {
        msg: String,
    },
    #[display(fmt = "Invalid Response: {:?}", reply)]
    InvalidResponse {
        reply: Reply,
    },
    Timeout,
    EncodingFailed,
    SendFailed,
    CallbackLost,
}

impl Error for AskError {}

impl From<SendError> for AskError {
    fn from(error: SendError) -> Self {
        match error {
            SendError::EncodingFailed => Self::EncodingFailed,
            SendError::SendFailed => Self::SendFailed,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Display)]
pub enum FileAskError {
    #[display(fmt = "{}", "_0")]
    GeneralAskFailed(AskError),

    #[display(fmt = "IO Error: {}", "_0")]
    #[serde(
        serialize_with = "over_there_utils::serializers::io_error::serialize",
        deserialize_with = "over_there_utils::serializers::io_error::deserialize"
    )]
    IoError(io::Error),

    #[display(fmt = "File signature changed: {}", id)]
    FileSignatureChanged { id: u32 },
}

impl Error for FileAskError {}

impl From<AskError> for FileAskError {
    fn from(error: AskError) -> Self {
        Self::GeneralAskFailed(error)
    }
}

impl From<io::Error> for FileAskError {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}

#[derive(Debug, Display)]
pub enum ExecAskError {
    #[display(fmt = "{}", "_0")]
    GeneralAskFailed(AskError),

    #[display(fmt = "IO Error: {}", "_0")]
    IoError(io::Error),

    FailedToKill,
}

impl Error for ExecAskError {}

impl From<AskError> for ExecAskError {
    fn from(error: AskError) -> Self {
        Self::GeneralAskFailed(error)
    }
}

impl From<io::Error> for ExecAskError {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}
