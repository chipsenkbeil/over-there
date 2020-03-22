use crate::Content;
use derive_more::Display;
use std::error::Error;
use std::io;

#[derive(Debug, Display, PartialEq, Eq)]
pub enum TellError {
    EncodingFailed,
    SendFailed,
}

impl Error for TellError {}

impl From<AskError> for Option<TellError> {
    fn from(error: AskError) -> Self {
        match error {
            AskError::EncodingFailed => Some(TellError::EncodingFailed),
            AskError::SendFailed => Some(TellError::SendFailed),
            _ => None,
        }
    }
}

#[derive(Debug, Display, PartialEq, Eq)]
pub enum AskError {
    #[display(fmt = "Failed: {}", msg)]
    Failure {
        msg: String,
    },
    #[display(fmt = "Invalid Response: {:?}", content)]
    InvalidResponse {
        content: Content,
    },
    Timeout,
    EncodingFailed,
    SendFailed,
    CallbackLost,
}

impl Error for AskError {}

impl From<TellError> for AskError {
    fn from(error: TellError) -> Self {
        match error {
            TellError::EncodingFailed => Self::EncodingFailed,
            TellError::SendFailed => Self::SendFailed,
        }
    }
}

#[derive(Debug, Display)]
pub enum FileAskError {
    #[display(fmt = "{}", "_0")]
    GeneralAskFailed(AskError),

    #[display(fmt = "IO Error: {}", "_0")]
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
