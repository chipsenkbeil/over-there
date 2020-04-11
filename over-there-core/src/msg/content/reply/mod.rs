mod batch;
mod capabilities;
mod custom;
mod forward;
mod generic_error;
mod internal_debug;
mod io;
mod sequence;
mod version;

pub use batch::*;
pub use capabilities::*;
pub use custom::*;
pub use forward::*;
pub use generic_error::*;
pub use internal_debug::*;
pub use io::*;
pub use sequence::*;
pub use version::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", content = "payload")]
pub enum Reply {
    // ------------------------------------------------------------------------
    // Heartbeats are used to ensure remote instances are alive
    #[serde(rename = "heartbeat_reply")]
    Heartbeat,

    // ------------------------------------------------------------------------
    // Version information to ensure that we don't have
    // conflicting functionality
    #[serde(rename = "version_reply")]
    Version(VersionArgs),

    // ------------------------------------------------------------------------
    // Capability information to convey what is available remotely, which
    // can differ based on enabled features at compile time
    #[serde(rename = "capabilities_reply")]
    Capabilities(CapabilitiesArgs),

    // ------------------------------------------------------------------------
    // Dir-based operations such as creating and listing entries
    /// This will be returned upon creating a directory
    #[serde(rename = "create_dir_reply")]
    DirCreated(DirCreatedArgs),

    /// This will be returned upon renaming a directory
    #[serde(rename = "rename_dir_reply")]
    DirRenamed(DirRenamedArgs),

    /// This will be returned upon removing a directory
    #[serde(rename = "remove_dir_reply")]
    DirRemoved(DirRemovedArgs),

    /// This will be returned upon collecting the list of files and directories
    /// at the provided path
    #[serde(rename = "list_dir_contents_reply")]
    DirContentsList(DirContentsListArgs),

    // ------------------------------------------------------------------------
    // File-based operations such as reading and writing
    /// This will be returned upon a file being opened or refreshed
    #[serde(rename = "open_file_reply")]
    FileOpened(FileOpenedArgs),

    /// This will be returned upon a file being closed
    #[serde(rename = "close_file_reply")]
    FileClosed(FileClosedArgs),

    /// This will be returned upon renaming a file
    #[serde(rename = "rename_unopened_file_reply")]
    UnopenedFileRenamed(UnopenedFileRenamedArgs),

    /// This will be returned upon renaming an open file
    #[serde(rename = "rename_file_reply")]
    FileRenamed(FileRenamedArgs),

    /// This will be returned upon removing a file
    #[serde(rename = "remove_unopened_file_reply")]
    UnopenedFileRemoved(UnopenedFileRemovedArgs),

    /// This will be returned upon removing an open file
    #[serde(rename = "remove_file_reply")]
    FileRemoved(FileRemovedArgs),

    /// This will be returned upon reading a file's contents
    #[serde(rename = "read_file_reply")]
    FileContents(FileContentsArgs),

    /// This will be returned upon writing a file's contents
    /// Contains the updated signature for the file
    #[serde(rename = "write_file_reply")]
    FileWritten(FileWrittenArgs),

    // ------------------------------------------------------------------------
    // Program execution operations such as running and streaming
    /// This will be returned upon starting a process on the server, indicating
    /// success and providing an id for sending stdin and receiving stdout/stderr
    #[serde(rename = "exec_proc_reply")]
    ProcStarted(ProcStartedArgs),

    /// This will be returned upon successfully writing to stdin
    #[serde(rename = "write_stdin_reply")]
    StdinWritten(StdinWrittenArgs),

    /// This will be returned upon receiving stdout from a remote process on
    /// the server, if enabled when first executing
    #[serde(rename = "get_stdout_reply")]
    StdoutContents(StdoutContentsArgs),

    /// This will be returned upon receiving stderr from a remote process on
    /// the server, if enabled when first executing
    #[serde(rename = "get_stderr_reply")]
    StderrContents(StderrContentsArgs),

    /// This will be returned upon attempting to kill a process
    // TODO: This is returned for two different types, killing a proc
    //       and requesting status; should I make a duplicate for the proc
    //       kill that has proc kill args?
    #[serde(rename = "kill_proc_reply")]
    ProcKilled(ProcKilledArgs),

    /// This will be returned reporting the status of the process, indicating
    /// if still running or if has completed (and the exit code)
    // TODO: This is returned for two different types, killing a proc
    //       and requesting status; should I make a duplicate for the proc
    //       kill that has proc kill args?
    #[serde(rename = "proc_status_reply")]
    ProcStatus(ProcStatusArgs),

    // ------------------------------------------------------------------------
    // Miscellaneous, adhoc messages
    /// This will be returned upon encountering an error during evaluation
    #[serde(rename = "error_reply")]
    Error(ReplyError),

    /// This will be returned upon successfully evaluating a sequence of operations
    #[serde(rename = "sequence_reply")]
    Sequence(SequenceArgs),

    /// This will be returned upon successfully evaluating a batch of operations in parallel
    #[serde(rename = "batch_reply")]
    Batch(BatchArgs),

    /// This will be sent to either the client or server and the msg will be
    /// passed along to the associated address (if possible)
    #[serde(rename = "forward_reply")]
    Forward(ForwardArgs),

    /// This will be sent in either direction to provide a custom content
    /// that would be evaluated through user-implemented handlers
    #[serde(rename = "custom_reply")]
    Custom(CustomArgs),

    /// For debugging purposes when needing to query the state of client/server
    #[serde(rename = "internal_debug_reply")]
    InternalDebug(InternalDebugArgs),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ReplyError {
    #[serde(rename = "generic_error")]
    Generic(GenericErrorArgs),

    #[serde(rename = "io_error")]
    Io(IoErrorArgs),

    #[serde(rename = "file_sig_changed_error")]
    FileSigChanged(FileSigChangedArgs),
}

impl From<String> for ReplyError {
    fn from(text: String) -> Self {
        Self::Generic(GenericErrorArgs::from(text))
    }
}

impl From<&str> for ReplyError {
    fn from(text: &str) -> Self {
        Self::from(String::from(text))
    }
}
