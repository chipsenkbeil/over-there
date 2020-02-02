pub mod capabilities;
pub mod custom;
pub mod error;
pub mod exec;
pub mod file;
pub mod forward;
pub mod version;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Content {
    // ------------------------------------------------------------------------
    // Heartbeats are used to ensure remote instances are alive
    Heartbeat,

    // ------------------------------------------------------------------------
    // Version information to ensure that we don't have
    // conflicting functionality
    DoGetVersion,
    Version(version::VersionArgs),

    // ------------------------------------------------------------------------
    // Capability information to convey what is available remotely, which
    // can differ based on enabled features at compile time
    DoGetCapabilities,
    Capabilities(capabilities::CapabilitiesArgs),

    // ------------------------------------------------------------------------
    // File-based operations such as reading and writing
    /// This will be sent to indicate the desire to list all files/directories
    /// at the provided path
    DoListDirContents(file::DoListDirContentsArgs),

    /// This will be returned upon collecting the list of files and directories
    /// at the provided path
    DirContentsList(file::DirContentsListArgs),

    /// This will be sent to indicate the desire to read/write a file
    DoOpenFile(file::DoOpenFileArgs),

    /// This will be returned upon a file being opened or refreshed
    FileOpened(file::FileOpenedArgs),

    /// This will be sent to indicate the desire to read a file's contents
    DoReadFile(file::DoReadFileArgs),

    /// This will be returned upon reading a file's contents
    FileContents(file::FileContentsArgs),

    /// This will be sent to indicate the desire to write a file's contents
    DoWriteFile(file::DoWriteFileArgs),

    /// This will be returned upon writing a file's contents
    /// Contains the updated signature for the file
    FileWritten(file::FileWrittenArgs),

    /// This will be returned upon encountering a generic IO error
    FileError(file::FileErrorArgs),

    /// If a file operation fails due to the signature changing,
    /// this will be returned
    FileSigChanged(file::FileSigChangedArgs),

    // ------------------------------------------------------------------------
    // Program execution operations such as running and streaming
    ExecRequest(exec::ExecRequestArgs),
    ExecStreamRequest(exec::ExecStreamRequestArgs),
    ExecExit(exec::ExecExitArgs),
    ExecStreamResponse(exec::ExecStreamResponseArgs),

    // ------------------------------------------------------------------------
    // Miscellaneous, adhoc messages
    /// This will be returned upon a generic error being encountered on the
    /// server (like an HTTP 500 error)
    Error(error::ErrorArgs),

    /// This will be sent to either the client or server and the msg will be
    /// passed along to the associated address (if possible)
    Forward(forward::ForwardArgs),

    /// This will be sent in either direction to provide a custom content
    /// that would be evaluated through user-implemented handlers
    Custom(custom::CustomArgs),
}
