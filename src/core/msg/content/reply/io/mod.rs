mod fs;
mod proc;

pub use fs::*;
pub use proc::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IoErrorArgs {
    pub description: String,
    pub os_code: Option<i32>,
    pub error_kind: SerErrorKind,
}

impl crate::core::SchemaInfo for IoErrorArgs {}

impl ToString for IoErrorArgs {
    fn to_string(&self) -> String {
        self.description.clone()
    }
}

impl Default for IoErrorArgs {
    fn default() -> Self {
        Self {
            description: Default::default(),
            os_code: Default::default(),
            error_kind: io::ErrorKind::Other.into(),
        }
    }
}

impl IoErrorArgs {
    pub fn invalid_file_id(id: u32) -> Self {
        Self {
            description: format!("No file open with id {}", id),
            error_kind: io::ErrorKind::InvalidInput.into(),
            os_code: None,
        }
    }

    pub fn invalid_proc_id(id: u32) -> Self {
        Self {
            description: format!("No process executed with id {}", id),
            error_kind: io::ErrorKind::InvalidInput.into(),
            os_code: None,
        }
    }

    pub fn pipe_unavailable() -> Self {
        Self {
            description: String::from("Resource unavailable"),
            error_kind: io::ErrorKind::BrokenPipe.into(),
            os_code: None,
        }
    }

    pub fn from_error_with_prefix(error: io::Error, prefix: &str) -> Self {
        let mut args = Self::from(error);

        args.description = format!("{}{}", prefix, args.description);

        args
    }
}

impl From<io::Error> for IoErrorArgs {
    fn from(error: io::Error) -> Self {
        let error_kind = error.kind();
        let os_code = error.raw_os_error();
        let description = format!("{}", error);

        Self {
            description,
            error_kind: error_kind.into(),
            os_code,
        }
    }
}

impl Into<io::Error> for IoErrorArgs {
    fn into(self) -> io::Error {
        if let Some(code) = self.os_code {
            io::Error::from_raw_os_error(code)
        } else {
            io::Error::new(self.error_kind.into(), self.description)
        }
    }
}

/// This is a hack for us to have both serde serialization and JSON schema
/// implemented for io::ErrorKind; we have custom serializer/deserializer for
/// serde but no way to provide a custom function for schema
#[derive(JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SerErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    Interrupted,
    Other,
    UnexpectedEof,

    /// For types that are added later that are not covered
    NonExhaustive,
}

impl crate::core::SchemaInfo for SerErrorKind {}

impl From<io::ErrorKind> for SerErrorKind {
    fn from(error_kind: io::ErrorKind) -> Self {
        match error_kind {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::ConnectionRefused => Self::ConnectionRefused,
            io::ErrorKind::ConnectionReset => Self::ConnectionReset,
            io::ErrorKind::ConnectionAborted => Self::ConnectionAborted,
            io::ErrorKind::NotConnected => Self::NotConnected,
            io::ErrorKind::AddrInUse => Self::AddrInUse,
            io::ErrorKind::AddrNotAvailable => Self::AddrNotAvailable,
            io::ErrorKind::BrokenPipe => Self::BrokenPipe,
            io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            io::ErrorKind::WouldBlock => Self::WouldBlock,
            io::ErrorKind::InvalidInput => Self::InvalidInput,
            io::ErrorKind::InvalidData => Self::InvalidData,
            io::ErrorKind::TimedOut => Self::TimedOut,
            io::ErrorKind::WriteZero => Self::WriteZero,
            io::ErrorKind::Interrupted => Self::Interrupted,
            io::ErrorKind::Other => Self::Other,
            io::ErrorKind::UnexpectedEof => Self::UnexpectedEof,
            _ => Self::NonExhaustive,
        }
    }
}

impl From<SerErrorKind> for io::ErrorKind {
    fn from(ser_error_kind: SerErrorKind) -> Self {
        match ser_error_kind {
            SerErrorKind::NotFound => Self::NotFound,
            SerErrorKind::PermissionDenied => Self::PermissionDenied,
            SerErrorKind::ConnectionRefused => Self::ConnectionRefused,
            SerErrorKind::ConnectionReset => Self::ConnectionReset,
            SerErrorKind::ConnectionAborted => Self::ConnectionAborted,
            SerErrorKind::NotConnected => Self::NotConnected,
            SerErrorKind::AddrInUse => Self::AddrInUse,
            SerErrorKind::AddrNotAvailable => Self::AddrNotAvailable,
            SerErrorKind::BrokenPipe => Self::BrokenPipe,
            SerErrorKind::AlreadyExists => Self::AlreadyExists,
            SerErrorKind::WouldBlock => Self::WouldBlock,
            SerErrorKind::InvalidInput => Self::InvalidInput,
            SerErrorKind::InvalidData => Self::InvalidData,
            SerErrorKind::TimedOut => Self::TimedOut,
            SerErrorKind::WriteZero => Self::WriteZero,
            SerErrorKind::Interrupted => Self::Interrupted,
            SerErrorKind::Other => Self::Other,
            SerErrorKind::UnexpectedEof => Self::UnexpectedEof,

            // Treat other types as other
            SerErrorKind::NonExhaustive => Self::Other,
        }
    }
}
