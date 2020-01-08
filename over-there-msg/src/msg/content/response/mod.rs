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
pub enum Response {
    Standard(standard::StandardResponse),

    #[cfg(feature = "custom")]
    Custom(custom::CustomResponse),

    #[cfg(feature = "exec")]
    Exec(exec::ExecResponse),

    #[cfg(feature = "forward")]
    Forward(forward::ForwardResponse),

    #[cfg(feature = "file-system")]
    FileSystem(file_system::FileSystemResponse),
}

impl From<standard::StandardResponse> for Response {
    fn from(r: standard::StandardResponse) -> Self {
        Self::Standard(r)
    }
}

#[cfg(feature = "custom")]
impl From<custom::CustomResponse> for Response {
    fn from(r: custom::CustomResponse) -> Self {
        Self::Custom(r)
    }
}

#[cfg(feature = "exec")]
impl From<exec::ExecResponse> for Response {
    fn from(r: exec::ExecResponse) -> Self {
        Self::Exec(r)
    }
}

#[cfg(feature = "forward")]
impl From<forward::ForwardResponse> for Response {
    fn from(r: forward::ForwardResponse) -> Self {
        Self::Forward(r)
    }
}

#[cfg(feature = "file-system")]
impl From<file_system::FileSystemResponse> for Response {
    fn from(r: file_system::FileSystemResponse) -> Self {
        Self::FileSystem(r)
    }
}
