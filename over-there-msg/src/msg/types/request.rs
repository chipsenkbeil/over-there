use super::Content;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Debug)]
pub enum StandardRequest {
    /// Make sure daemon is alive
    HeartbeatRequest,

    /// Request version
    VersionRequest,

    /// Request capabilities
    CapabilitiesRquest,

    /// TODO: Think of format for hopping from one instance to another
    ///       in case of client -> server 1 -> server 2
    ///
    /// Server 2 Address, Message to forward
    ForwardRequest(String, Box<dyn Content>),

    /// Key-value map for custom requests
    ///
    /// Args: Map
    CustomRequest(HashMap<String, String>),
}

#[typetag::serde]
impl Content for StandardRequest {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FileRequest {
    /// List all files, directories, etc. at a path
    ///
    /// Path
    ListFilesRequest(String),

    /// Write the contents of a file
    ///
    /// Path, Contents
    WriteFileRequest(String, String),

    /// Read the contents of a file
    ///
    /// Path, Start (base 0), Total Bytes
    ReadFileRequest(String, u32, u32),
}

#[typetag::serde]
impl Content for FileRequest {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ExecRequest {
    /// Execute a command, potentially returning the completed output
    ///
    /// Args: Command, Args, WantStdOut, WantStdErr
    ExecRequest(String, Vec<String>, bool, bool),

    /// Execute a command, potentially streaming the live output
    ///
    /// Command, Args, WantStdOut, WantStdErr
    ExecStreamRequest(String, Vec<String>, bool, bool),
}

#[typetag::serde]
impl Content for ExecRequest {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
