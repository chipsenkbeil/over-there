use super::Content;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Debug)]
pub enum StandardResponse {
    /// Report alive status
    HeartbeatResponse,

    /// Report version
    VersionResponse(String),

    /// Report capabilities
    CapabilitiesResponse(Vec<String>),

    /// Generic error reponse used upon failing
    ///
    /// Error Message
    ErrorResponse(String),

    /// TODO: Think of format for hopping from one instance to another
    ///       in case of client -> server 1 -> server 2
    ///
    /// Client Address, Message to pass back
    ForwardResponse(String, Box<dyn Content>),

    /// Key-value map for custom responses
    ///
    /// Args: Map
    CustomResponse(HashMap<String, String>),
}

#[typetag::serde]
impl Content for StandardResponse {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FileResponse {
    /// List all files, directories, etc. at a path
    ///
    /// Paths
    ListFilesResponse(Vec<String>),

    /// Write the contents of a file
    ///
    /// Bytes written
    WriteFileResponse(u32),

    /// Read the contents of a file
    ///
    /// Bytes read
    ReadFileResponse(Vec<u8>),
}

#[typetag::serde]
impl Content for FileResponse {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ExecResponse {
    /// Execute a command, potentially returning the completed output
    ///
    /// ErrCode, StdOut, StdErr
    ExecResponse(u32, Option<String>, Option<String>),

    /// Execute a command, potentially streaming the live output
    ///
    /// ErrCode (none if still running), StdOut, StdErr
    ExecStreamResponse(Option<u32>, Option<String>, Option<String>),
}

#[typetag::serde]
impl Content for ExecResponse {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
