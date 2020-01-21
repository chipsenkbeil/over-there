use super::Msg;
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
    Error {
        msg: String,
    },
    Forward {
        address: SocketAddr,
        msg: Box<Msg>,
    },

    // ------------------------------------------------------------------------
    // File-based operations such as reading and writing
    ListFilesRequest {
        path: String,
    },
    WriteFileRequest {
        path: String,
        contents: String,
    },
    ReadFileRequest {
        path: String,
        start: u32,
        size: u32,
    },
    ListFilesResponse {
        paths: Vec<String>,
    },
    WriteFileResponse {
        bytes_written: u32,
    },
    ReadFileResponse {
        data: Vec<u8>,
    },

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
