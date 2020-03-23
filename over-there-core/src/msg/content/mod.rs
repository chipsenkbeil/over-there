mod capabilities;
mod custom;
mod error;
mod forward;
mod internal_debug;
mod io;
mod version;

pub use capabilities::*;
pub use custom::*;
pub use error::*;
pub use forward::*;
pub use internal_debug::*;
pub use io::*;
pub use version::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Content {
    // ------------------------------------------------------------------------
    // Heartbeats are used to ensure remote instances are alive
    Heartbeat,

    // ------------------------------------------------------------------------
    // Version information to ensure that we don't have
    // conflicting functionality
    DoGetVersion,
    Version(VersionArgs),

    // ------------------------------------------------------------------------
    // Capability information to convey what is available remotely, which
    // can differ based on enabled features at compile time
    DoGetCapabilities,
    Capabilities(CapabilitiesArgs),

    // ------------------------------------------------------------------------
    // Dir-based operations such as creating and listing entries
    /// This will be sent to indicate the desire to create a new directory
    DoCreateDir(DoCreateDirArgs),

    /// This will be returned upon creating a directory
    DirCreated(DirCreatedArgs),

    /// This will be sent to indicate the desire to rename a directory
    DoRenameDir(DoRenameDirArgs),

    /// This will be returned upon renaming a directory
    DirRenamed(DirRenamedArgs),

    /// This will be sent to indicate the desire to remove a directory
    DoRemoveDir(DoRemoveDirArgs),

    /// This will be returned upon removing a directory
    DirRemoved(DirRemovedArgs),

    /// This will be sent to indicate the desire to list all files/directories
    /// at the provided path
    DoListDirContents(DoListDirContentsArgs),

    /// This will be returned upon collecting the list of files and directories
    /// at the provided path
    DirContentsList(DirContentsListArgs),

    // ------------------------------------------------------------------------
    // File-based operations such as reading and writing
    /// This will be sent to indicate the desire to read/write a file,
    /// and can also be used to retrieve an already-open file's id/sig
    DoOpenFile(DoOpenFileArgs),

    /// This will be returned upon a file being opened or refreshed
    FileOpened(FileOpenedArgs),

    /// This will be sent to indicate the desire to close an open file
    DoCloseFile(DoCloseFileArgs),

    /// This will be returned upon a file being closed
    FileClosed(FileClosedArgs),

    /// This will be sent to indicate the desire to rename a file
    DoRenameUnopenedFile(DoRenameUnopenedFileArgs),

    /// This will be returned upon renaming a file
    UnopenedFileRenamed(UnopenedFileRenamedArgs),

    /// This will be sent to indicate the desire to rename an open file
    DoRenameFile(DoRenameFileArgs),

    /// This will be returned upon renaming an open file
    FileRenamed(FileRenamedArgs),

    /// This will be sent to indicate the desire to remove a file
    DoRemoveUnopenedFile(DoRemoveUnopenedFileArgs),

    /// This will be returned upon removing a file
    UnopenedFileRemoved(UnopenedFileRemovedArgs),

    /// This will be sent to indicate the desire to remove an open file
    DoRemoveFile(DoRemoveFileArgs),

    /// This will be returned upon removing an open file
    FileRemoved(FileRemovedArgs),

    /// This will be sent to indicate the desire to read a file's contents
    DoReadFile(DoReadFileArgs),

    /// This will be returned upon reading a file's contents
    FileContents(FileContentsArgs),

    /// This will be sent to indicate the desire to write a file's contents
    DoWriteFile(DoWriteFileArgs),

    /// This will be returned upon writing a file's contents
    /// Contains the updated signature for the file
    FileWritten(FileWrittenArgs),

    /// If a file operation fails due to the signature changing,
    /// this will be returned
    FileSigChanged(FileSigChangedArgs),

    // ------------------------------------------------------------------------
    // Program execution operations such as running and streaming
    /// This will be sent to execute a remote proccess on the server
    DoExecProc(DoExecProcArgs),

    /// This will be returned upon starting a process on the server, indicating
    /// success and providing an id for sending stdin and receiving stdout/stderr
    ProcStarted(ProcStartedArgs),

    /// This will be sent to feed input to a remote process on the server, if
    /// enabled when first executing
    DoWriteStdin(DoWriteStdinArgs),

    /// This will be returned upon successfully writing to stdin
    StdinWritten(StdinWrittenArgs),

    /// This will be sent to request all stdout for a remote process on
    /// the server since the last request was made
    DoGetStdout(DoGetStdoutArgs),

    /// This will be returned upon receiving stdout from a remote process on
    /// the server, if enabled when first executing
    StdoutContents(StdoutContentsArgs),

    /// This will be sent to request all stderr for a remote process on
    /// the server since the last request was made
    DoGetStderr(DoGetStderrArgs),

    /// This will be returned upon receiving stderr from a remote process on
    /// the server, if enabled when first executing
    StderrContents(StderrContentsArgs),

    /// This will be sent to kill a remote process on the server
    DoKillProc(DoKillProcArgs),

    /// This will be sent to request the status of a running process on
    /// the server
    DoGetProcStatus(DoGetProcStatusArgs),

    /// This will be returned reporting the status of the process, indicating
    /// if still running or if has completed (and the exit code)
    ProcStatus(ProcStatusArgs),

    // ------------------------------------------------------------------------
    // Miscellaneous, adhoc messages
    /// This will be returned upon encountering a generic IO error
    IoError(IoErrorArgs),

    /// This will be returned upon a generic error being encountered on the
    /// server (like an HTTP 500 error)
    Error(ErrorArgs),

    /// This will be sent to either the client or server and the msg will be
    /// passed along to the associated address (if possible)
    Forward(ForwardArgs),

    /// This will be sent in either direction to provide a custom content
    /// that would be evaluated through user-implemented handlers
    Custom(CustomArgs),

    /// For debugging purposes when needing to query the state of client/server
    InternalDebug(InternalDebugArgs),
}
