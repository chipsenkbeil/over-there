use crate::Content;
use over_there_derive::Error;
use std::io;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TellError {
    EncodingFailed,
    SendFailed,
}

impl From<AskError> for Option<TellError> {
    fn from(error: AskError) -> Self {
        match error {
            AskError::EncodingFailed => Some(TellError::EncodingFailed),
            AskError::SendFailed => Some(TellError::SendFailed),
            _ => None,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AskError {
    Failure { msg: String },
    InvalidResponse { content: Content },
    Timeout,
    EncodingFailed,
    SendFailed,
}

impl From<TellError> for AskError {
    fn from(error: TellError) -> Self {
        match error {
            TellError::EncodingFailed => Self::EncodingFailed,
            TellError::SendFailed => Self::SendFailed,
        }
    }
}

#[derive(Debug, Error)]
pub enum FileAskError {
    GeneralAskFailed(AskError),
    IoError(io::Error),
    FileSignatureChanged { id: u32 },
}

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

#[derive(Debug, Error)]
pub enum ExecAskError {
    GeneralAskFailed(AskError),
    IoError(io::Error),
    FailedToKill,
}

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
