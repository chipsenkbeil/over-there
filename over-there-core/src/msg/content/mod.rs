pub mod capabilities;
pub mod custom;
pub mod error;
pub mod forward;
pub mod io;
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
    // Dir-based operations such as creating and listing entries
    /// This will be sent to indicate the desire to create a new directory
    DoCreateDir(io::fs::DoCreateDirArgs),

    /// This will be returned upon creating a directory
    DirCreated(io::fs::DirCreatedArgs),

    /// This will be sent to indicate the desire to rename a directory
    DoRenameDir(io::fs::DoRenameDirArgs),

    /// This will be returned upon renaming a directory
    DirRenamed(io::fs::DirRenamedArgs),

    /// This will be sent to indicate the desire to remove a directory
    DoRemoveDir(io::fs::DoRemoveDirArgs),

    /// This will be returned upon removing a directory
    DirRemoved(io::fs::DirRemovedArgs),

    /// This will be sent to indicate the desire to list all files/directories
    /// at the provided path
    DoListDirContents(io::fs::DoListDirContentsArgs),

    /// This will be returned upon collecting the list of files and directories
    /// at the provided path
    DirContentsList(io::fs::DirContentsListArgs),

    // ------------------------------------------------------------------------
    // File-based operations such as reading and writing
    /// This will be sent to indicate the desire to read/write a file,
    /// and can also be used to retrieve an already-open file's id/sig
    DoOpenFile(io::fs::DoOpenFileArgs),

    /// This will be returned upon a file being opened or refreshed
    FileOpened(io::fs::FileOpenedArgs),

    /// This will be sent to indicate the desire to close an open file
    DoCloseFile(io::fs::DoCloseFileArgs),

    /// This will be returned upon a file being closed
    FileClosed(io::fs::FileClosedArgs),

    /// This will be sent to indicate the desire to rename a file
    DoRenameUnopenedFile(io::fs::DoRenameUnopenedFileArgs),

    /// This will be returned upon renaming a file
    UnopenedFileRenamed(io::fs::UnopenedFileRenamedArgs),

    /// This will be sent to indicate the desire to rename an open file
    DoRenameFile(io::fs::DoRenameFileArgs),

    /// This will be returned upon renaming an open file
    FileRenamed(io::fs::FileRenamedArgs),

    /// This will be sent to indicate the desire to remove a file
    DoRemoveUnopenedFile(io::fs::DoRemoveUnopenedFileArgs),

    /// This will be returned upon removing a file
    UnopenedFileRemoved(io::fs::UnopenedFileRemovedArgs),

    /// This will be sent to indicate the desire to remove an open file
    DoRemoveFile(io::fs::DoRemoveFileArgs),

    /// This will be returned upon removing an open file
    FileRemoved(io::fs::FileRemovedArgs),

    /// This will be sent to indicate the desire to read a file's contents
    DoReadFile(io::fs::DoReadFileArgs),

    /// This will be returned upon reading a file's contents
    FileContents(io::fs::FileContentsArgs),

    /// This will be sent to indicate the desire to write a file's contents
    DoWriteFile(io::fs::DoWriteFileArgs),

    /// This will be returned upon writing a file's contents
    /// Contains the updated signature for the file
    FileWritten(io::fs::FileWrittenArgs),

    /// If a file operation fails due to the signature changing,
    /// this will be returned
    FileSigChanged(io::fs::FileSigChangedArgs),

    // ------------------------------------------------------------------------
    // Program execution operations such as running and streaming
    /// This will be sent to execute a remote proccess on the server
    DoExecProc(io::proc::DoExecProcArgs),

    /// This will be returned upon starting a process on the server, indicating
    /// success and providing an id for sending stdin and receiving stdout/stderr
    ProcStarted(io::proc::ProcStartedArgs),

    /// This will be sent to feed input to a remote process on the server, if
    /// enabled when first executing
    DoWriteStdin(io::proc::DoWriteStdinArgs),

    /// This will be returned upon successfully writing to stdin
    StdinWritten(io::proc::StdinWrittenArgs),

    /// This will be sent to request all stdout for a remote process on
    /// the server since the last request was made
    DoGetStdout(io::proc::DoGetStdoutArgs),

    /// This will be returned upon receiving stdout from a remote process on
    /// the server, if enabled when first executing
    StdoutContents(io::proc::StdoutContentsArgs),

    /// This will be sent to request all stderr for a remote process on
    /// the server since the last request was made
    DoGetStderr(io::proc::DoGetStderrArgs),

    /// This will be returned upon receiving stderr from a remote process on
    /// the server, if enabled when first executing
    StderrContents(io::proc::StderrContentsArgs),

    /// This will be sent to kill a remote process on the server
    DoKillProc(io::proc::DoKillProcArgs),

    /// This will be sent to request the status of a running process on
    /// the server
    DoGetProcStatus(io::proc::DoGetProcStatusArgs),

    /// This will be returned reporting the status of the process, indicating
    /// if still running or if has completed (and the exit code)
    ProcStatus(io::proc::ProcStatusArgs),

    // ------------------------------------------------------------------------
    // Miscellaneous, adhoc messages
    /// This will be returned upon encountering a generic IO error
    IoError(io::IoErrorArgs),

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
