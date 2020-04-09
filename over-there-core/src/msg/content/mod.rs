mod batch;
mod capabilities;
mod custom;
mod error;
mod forward;
mod internal_debug;
mod io;
mod sequence;
mod transform;
mod version;

pub use batch::*;
pub use capabilities::*;
pub use custom::*;
pub use error::*;
pub use forward::*;
pub use internal_debug::*;
pub use io::*;
pub use sequence::*;
pub use transform::*;
pub use version::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum Content {
    Request(Request),
    Response(Response),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum Request {
    // ------------------------------------------------------------------------
    // Heartbeats are used to ensure remote instances are alive
    #[serde(rename = "heartbeat_request")]
    Heartbeat,

    // ------------------------------------------------------------------------
    // Version information to ensure that we don't have
    // conflicting functionality
    #[serde(rename = "version_request")]
    Version,

    // ------------------------------------------------------------------------
    // Capability information to convey what is available remotely, which
    // can differ based on enabled features at compile time
    #[serde(rename = "capabilities_request")]
    Capabilities,

    // ------------------------------------------------------------------------
    // Dir-based operations such as creating and listing entries
    /// This will be sent to indicate the desire to create a new directory
    #[serde(rename = "create_dir_request")]
    DoCreateDir(DoCreateDirArgs),

    /// This will be sent to indicate the desire to rename a directory
    #[serde(rename = "rename_dir_request")]
    DoRenameDir(DoRenameDirArgs),

    /// This will be sent to indicate the desire to remove a directory
    #[serde(rename = "remove_dir_request")]
    DoRemoveDir(DoRemoveDirArgs),

    /// This will be sent to indicate the desire to list all files/directories
    /// at the provided path
    #[serde(rename = "list_dir_contents_request")]
    DoListDirContents(DoListDirContentsArgs),

    // ------------------------------------------------------------------------
    // File-based operations such as reading and writing
    /// This will be sent to indicate the desire to read/write a file,
    /// and can also be used to retrieve an already-open file's id/sig
    #[serde(rename = "open_file_request")]
    DoOpenFile(DoOpenFileArgs),

    /// This will be sent to indicate the desire to close an open file
    #[serde(rename = "close_file_request")]
    DoCloseFile(DoCloseFileArgs),

    /// This will be sent to indicate the desire to rename a file
    #[serde(rename = "rename_unopened_file_request")]
    DoRenameUnopenedFile(DoRenameUnopenedFileArgs),

    /// This will be sent to indicate the desire to rename an open file
    #[serde(rename = "rename_file_request")]
    DoRenameFile(DoRenameFileArgs),

    /// This will be sent to indicate the desire to remove a file
    #[serde(rename = "remove_unopened_file_request")]
    DoRemoveUnopenedFile(DoRemoveUnopenedFileArgs),

    /// This will be sent to indicate the desire to remove an open file
    #[serde(rename = "remove_file_request")]
    DoRemoveFile(DoRemoveFileArgs),

    /// This will be sent to indicate the desire to read a file's contents
    #[serde(rename = "read_file_request")]
    DoReadFile(DoReadFileArgs),

    /// This will be sent to indicate the desire to write a file's contents
    #[serde(rename = "write_file_request")]
    DoWriteFile(DoWriteFileArgs),

    // ------------------------------------------------------------------------
    // Program execution operations such as running and streaming
    /// This will be sent to execute a remote proccess on the server
    #[serde(rename = "exec_proc_request")]
    DoExecProc(DoExecProcArgs),

    /// This will be sent to feed input to a remote process on the server, if
    /// enabled when first executing
    #[serde(rename = "write_stdin_request")]
    DoWriteStdin(DoWriteStdinArgs),

    /// This will be sent to request all stdout for a remote process on
    /// the server since the last request was made
    #[serde(rename = "get_stdout_request")]
    DoGetStdout(DoGetStdoutArgs),

    /// This will be sent to request all stderr for a remote process on
    /// the server since the last request was made
    #[serde(rename = "get_stderr_request")]
    DoGetStderr(DoGetStderrArgs),

    /// This will be sent to kill a remote process on the server
    #[serde(rename = "kill_proc_request")]
    DoKillProc(DoKillProcArgs),

    /// This will be sent to request the status of a running process on
    /// the server
    #[serde(rename = "get_proc_status_request")]
    DoGetProcStatus(DoGetProcStatusArgs),

    // ------------------------------------------------------------------------
    // Miscellaneous, adhoc messages
    /// This will be sent to execute a collection of operations sequentially
    #[serde(rename = "sequence_request")]
    DoSequence(DoSequenceArgs),

    /// This will be sent to execute a collection of operations in parallel
    #[serde(rename = "batch_request")]
    DoBatch(DoBatchArgs),

    /// This will be sent to either the client or server and the msg will be
    /// passed along to the associated address (if possible)
    #[serde(rename = "forward_request")]
    Forward(ForwardArgs),

    /// This will be sent in either direction to provide a custom content
    /// that would be evaluated through user-implemented handlers
    #[serde(rename = "custom_request")]
    Custom(CustomArgs),

    /// For debugging purposes when needing to query the state of client/server
    #[serde(rename = "internal_debug_request")]
    InternalDebug(InternalDebugArgs),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum Response {
    // ------------------------------------------------------------------------
    // Heartbeats are used to ensure remote instances are alive
    #[serde(rename = "heartbeat_response")]
    Heartbeat,

    // ------------------------------------------------------------------------
    // Version information to ensure that we don't have
    // conflicting functionality
    #[serde(rename = "version_response")]
    Version(VersionArgs),

    // ------------------------------------------------------------------------
    // Capability information to convey what is available remotely, which
    // can differ based on enabled features at compile time
    #[serde(rename = "capabilities_response")]
    Capabilities(CapabilitiesArgs),

    // ------------------------------------------------------------------------
    // Dir-based operations such as creating and listing entries
    /// This will be returned upon creating a directory
    #[serde(rename = "create_dir_response")]
    DirCreated(DirCreatedArgs),

    /// This will be returned upon renaming a directory
    #[serde(rename = "rename_dir_response")]
    DirRenamed(DirRenamedArgs),

    /// This will be returned upon removing a directory
    #[serde(rename = "remove_dir_response")]
    DirRemoved(DirRemovedArgs),

    /// This will be returned upon collecting the list of files and directories
    /// at the provided path
    #[serde(rename = "list_dir_contents_response")]
    DirContentsList(DirContentsListArgs),

    // ------------------------------------------------------------------------
    // File-based operations such as reading and writing
    /// This will be returned upon a file being opened or refreshed
    #[serde(rename = "open_file_response")]
    FileOpened(FileOpenedArgs),

    /// This will be returned upon a file being closed
    #[serde(rename = "close_file_response")]
    FileClosed(FileClosedArgs),

    /// This will be returned upon renaming a file
    #[serde(rename = "rename_unopened_file_response")]
    UnopenedFileRenamed(UnopenedFileRenamedArgs),

    /// This will be returned upon renaming an open file
    #[serde(rename = "rename_file_response")]
    FileRenamed(FileRenamedArgs),

    /// This will be returned upon removing a file
    #[serde(rename = "remove_unopened_file_response")]
    UnopenedFileRemoved(UnopenedFileRemovedArgs),

    /// This will be returned upon removing an open file
    #[serde(rename = "remove_file_response")]
    FileRemoved(FileRemovedArgs),

    /// This will be returned upon reading a file's contents
    #[serde(rename = "read_file_response")]
    FileContents(FileContentsArgs),

    /// This will be returned upon writing a file's contents
    /// Contains the updated signature for the file
    #[serde(rename = "write_file_response")]
    FileWritten(FileWrittenArgs),

    /// If a file operation fails due to the signature changing,
    /// this will be returned
    // TODO: How do I handle these cases? These are errors, so do
    //       the arguments of all variants need to support Result
    //       which can contain error types?
    #[serde(rename = "file_sig_changed")]
    FileSigChanged(FileSigChangedArgs),

    // ------------------------------------------------------------------------
    // Program execution operations such as running and streaming
    /// This will be returned upon starting a process on the server, indicating
    /// success and providing an id for sending stdin and receiving stdout/stderr
    #[serde(rename = "exec_proc_response")]
    ProcStarted(ProcStartedArgs),

    /// This will be returned upon successfully writing to stdin
    #[serde(rename = "write_stdin_response")]
    StdinWritten(StdinWrittenArgs),

    /// This will be returned upon receiving stdout from a remote process on
    /// the server, if enabled when first executing
    #[serde(rename = "get_stdout_response")]
    StdoutContents(StdoutContentsArgs),

    /// This will be returned upon receiving stderr from a remote process on
    /// the server, if enabled when first executing
    #[serde(rename = "get_stderr_response")]
    StderrContents(StderrContentsArgs),

    /// This will be returned reporting the status of the process, indicating
    /// if still running or if has completed (and the exit code)
    // TODO: This is returned for two different types, killing a proc
    //       and requesting status; should I make a duplicate for the proc
    //       kill that has proc kill args?
    #[serde(rename = "get_proc_status_response")]
    ProcStatus(ProcStatusArgs),

    // ------------------------------------------------------------------------
    // Miscellaneous, adhoc messages
    /// This will be returned upon encountering a generic IO error
    // TODO: Similar to the sig mismatch error above, this is also an error
    //       that could be returned for any of these responses; do we need to
    //       separate these into their own Error enum that is used as the
    //       error in a response? Do we change things like Forward(ForwardArgs) ->
    //       Forward(Result<ForwardArgs, Error>)?
    #[serde(rename = "io_error")]
    IoError(IoErrorArgs),

    /// This will be returned upon a generic error being encountered on the
    /// server (like an HTTP 500 error)
    // TODO: Similar to the sig mismatch error above, this is also an error
    //       that could be returned for any of these responses; do we need to
    //       separate these into their own Error enum that is used as the
    //       error in a response? Do we change things like Forward(ForwardArgs) ->
    //       Forward(Result<ForwardArgs, Error>)?
    #[serde(rename = "error")]
    Error(ErrorArgs),

    /// This will be returned upon successfully evaluating a sequence of operations
    #[serde(rename = "sequence_response")]
    SequenceResults(SequenceResultsArgs),

    /// This will be returned upon successfully evaluating a batch of operations in parallel
    #[serde(rename = "batch_response")]
    BatchResults(BatchResultsArgs),

    /// This will be sent to either the client or server and the msg will be
    /// passed along to the associated address (if possible)
    #[serde(rename = "forward_response")]
    Forward(ForwardArgs),

    /// This will be sent in either direction to provide a custom content
    /// that would be evaluated through user-implemented handlers
    #[serde(rename = "custom_response")]
    Custom(CustomArgs),

    /// For debugging purposes when needing to query the state of client/server
    #[serde(rename = "internal_debug_response")]
    InternalDebug(InternalDebugArgs),
}
