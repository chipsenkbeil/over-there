mod batch;
mod capabilities;
mod custom;
mod forward;
mod internal_debug;
mod io;
mod sequence;
mod transform;

pub use batch::*;
pub use capabilities::*;
pub use custom::*;
pub use forward::*;
pub use internal_debug::*;
pub use io::*;
pub use sequence::*;
pub use transform::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
    CreateDir(CreateDirArgs),

    /// This will be sent to indicate the desire to rename a directory
    #[serde(rename = "rename_dir_request")]
    RenameDir(RenameDirArgs),

    /// This will be sent to indicate the desire to remove a directory
    #[serde(rename = "remove_dir_request")]
    RemoveDir(RemoveDirArgs),

    /// This will be sent to indicate the desire to list all files/directories
    /// at the provided path
    #[serde(rename = "list_dir_contents_request")]
    ListDirContents(ListDirContentsArgs),

    // ------------------------------------------------------------------------
    // File-based operations such as reading and writing
    /// This will be sent to indicate the desire to read/write a file,
    /// and can also be used to retrieve an already-open file's id/sig
    #[serde(rename = "open_file_request")]
    OpenFile(OpenFileArgs),

    /// This will be sent to indicate the desire to close an open file
    #[serde(rename = "close_file_request")]
    CloseFile(CloseFileArgs),

    /// This will be sent to indicate the desire to rename a file
    #[serde(rename = "rename_unopened_file_request")]
    RenameUnopenedFile(RenameUnopenedFileArgs),

    /// This will be sent to indicate the desire to rename an open file
    #[serde(rename = "rename_file_request")]
    RenameFile(RenameFileArgs),

    /// This will be sent to indicate the desire to remove a file
    #[serde(rename = "remove_unopened_file_request")]
    RemoveUnopenedFile(RemoveUnopenedFileArgs),

    /// This will be sent to indicate the desire to remove an open file
    #[serde(rename = "remove_file_request")]
    RemoveFile(RemoveFileArgs),

    /// This will be sent to indicate the desire to read a file's contents
    #[serde(rename = "read_file_request")]
    ReadFile(ReadFileArgs),

    /// This will be sent to indicate the desire to write a file's contents
    #[serde(rename = "write_file_request")]
    WriteFile(WriteFileArgs),

    // ------------------------------------------------------------------------
    // Program execution operations such as running and streaming
    /// This will be sent to execute a remote proccess on the server
    #[serde(rename = "exec_proc_request")]
    ExecProc(ExecProcArgs),

    /// This will be sent to feed input to a remote process on the server, if
    /// enabled when first executing
    #[serde(rename = "write_proc_stdin_request")]
    WriteProcStdin(WriteProcStdinArgs),

    /// This will be sent to request all stdout for a remote process on
    /// the server since the last request was made
    #[serde(rename = "read_proc_stdout_request")]
    ReadProcStdout(ReadProcStdoutArgs),

    /// This will be sent to request all stderr for a remote process on
    /// the server since the last request was made
    #[serde(rename = "read_proc_stderr_request")]
    ReadProcStderr(ReadProcStderrArgs),

    /// This will be sent to kill a remote process on the server
    #[serde(rename = "kill_proc_request")]
    KillProc(KillProcArgs),

    /// This will be sent to request the status of a running process on
    /// the server
    #[serde(rename = "read_proc_status_request")]
    ReadProcStatus(ReadProcStatusArgs),

    // ------------------------------------------------------------------------
    // Miscellaneous, adhoc messages
    /// This will be sent to execute a collection of operations sequentially
    #[serde(rename = "sequence_request")]
    Sequence(SequenceArgs),

    /// This will be sent to execute a collection of operations in parallel
    #[serde(rename = "batch_request")]
    Batch(BatchArgs),

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

impl Request {
    /// Converts a request into a lazily transformed request using the
    /// provided rules as transformation specifications
    pub fn into_lazily_transformed(
        self,
        rules: Vec<TransformRule>,
    ) -> LazilyTransformedRequest {
        LazilyTransformedRequest::new(self, rules)
    }
}

impl crate::SchemaInfo for Request {}
