use super::Msg;
use over_there_utils::serializers;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use strum_macros::{Display, EnumDiscriminants, EnumString};

#[derive(EnumDiscriminants, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[strum_discriminants(name(ContentType))]
#[strum_discriminants(derive(Display, EnumString, Hash))]
pub enum Content {
    // ------------------------------------------------------------------------
    // Heartbeats are used to ensure remote instances are alive
    HeartbeatRequest,
    HeartbeatResponse,

    // ------------------------------------------------------------------------
    // Version information to ensure that we don't have
    // conflicting functionality
    VersionRequest,
    VersionResponse {
        version: String,
    },

    // ------------------------------------------------------------------------
    // Capability information to convey what is available remotely, which
    // can differ based on enabled features at compile time
    CapabilitiesRequest,
    CapabilitiesResponse {
        capabilities: Vec<String>,
    },

    // ------------------------------------------------------------------------
    // Miscellaneous, adhoc messages
    /// This will be returned upon a generic error being encountered on the
    /// server (like an HTTP 500 error)
    Error {
        msg: String,
    },

    /// This will be sent to either the client or server and the msg will be
    /// passed along to the associated address (if possible)
    Forward {
        address: SocketAddr,
        msg: Box<Msg>,
    },

    // ------------------------------------------------------------------------
    // File-based operations such as reading and writing
    /// This will be sent to indicate the desire to list all files/directories
    /// at the provided path
    FileDoList {
        path: String,
    },

    /// This will be returned upon collecting the list of files and directories
    /// at the provided path
    FileList {
        entries: Vec<()>,
    },

    /// This will be sent to indicate the desire to read/write a file
    FileDoOpen {
        path: String,
        create_if_missing: bool,
        write_access: bool,
    },

    /// This will be returned upon a file being opened or refreshed
    FileOpened {
        id: u32,
        sig: u32,
    },

    /// This will be sent to indicate the desire to read a file's contents
    FileDoRead {
        id: u32,
        sig: u32,
    },

    /// This will be returned upon reading a file's contents
    FileContents {
        data: Vec<u8>,
    },

    /// This will be sent to indicate the desire to write a file's contents
    FileDoWrite {
        id: u32,
        sig: u32,
        data: Vec<u8>,
    },

    /// This will be returned upon writing a file's contents
    /// Contains the updated signature for the file
    FileWritten {
        sig: u32,
    },

    /// This will be returned upon encountering a generic IO error
    FileError {
        description: String,
        #[serde(
            serialize_with = "serializers::error_kind::serialize",
            deserialize_with = "serializers::error_kind::deserialize"
        )]
        error_kind: std::io::ErrorKind,
    },

    /// If a file operation fails due to the signature changing,
    /// this will be returned
    FileSigChanged,

    // ------------------------------------------------------------------------
    // Program execution operations such as running and streaming
    ExecRequest {
        command: String,
        args: Vec<String>,
        capture_std_out: bool,
        capture_std_err: bool,
    },
    ExecStreamRequest {
        command: String,
        args: Vec<String>,
        capture_std_out: bool,
        capture_std_err: bool,
    },
    ExecExit {
        exit_code: u32,
        std_out: Option<String>,
        std_err: Option<String>,
    },
    ExecStreamResponse {
        exit_code: Option<u32>,
        std_out: Option<String>,
        std_err: Option<String>,
    },
}
