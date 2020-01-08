#[cfg(feature = "custom")]
pub mod custom;
#[cfg(feature = "exec")]
pub mod exec;
#[cfg(feature = "file-system")]
pub mod file_system;
#[cfg(feature = "forward")]
pub mod forward;
pub mod standard;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Request {
    Standard(standard::StandardRequest),

    #[cfg(feature = "custom")]
    Custom(custom::CustomRequest),

    #[cfg(feature = "exec")]
    Exec(exec::ExecRequest),

    #[cfg(feature = "forward")]
    Forward(forward::ForwardRequest),

    #[cfg(feature = "file-system")]
    FileSystem(file_system::FileSystemRequest),
}

impl From<standard::StandardRequest> for Request {
    fn from(r: standard::StandardRequest) -> Self {
        Self::Standard(r)
    }
}

#[cfg(feature = "custom")]
impl From<custom::CustomRequest> for Request {
    fn from(r: custom::CustomRequest) -> Self {
        Self::Custom(r)
    }
}

#[cfg(feature = "exec")]
impl From<exec::ExecRequest> for Request {
    fn from(r: exec::ExecRequest) -> Self {
        Self::Exec(r)
    }
}

#[cfg(feature = "forward")]
impl From<forward::ForwardRequest> for Request {
    fn from(r: forward::ForwardRequest) -> Self {
        Self::Forward(r)
    }
}

#[cfg(feature = "file-system")]
impl From<file_system::FileSystemRequest> for Request {
    fn from(r: file_system::FileSystemRequest) -> Self {
        Self::FileSystem(r)
    }
}
