use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum FileSystemRequest {
    /// List all files, directories, etc. at a path
    ListFilesRequest { path: String },

    /// Write the contents of a file
    WriteFileRequest { path: String, contents: String },

    /// Read the contents of a file
    ReadFileRequest { path: String, start: u32, size: u32 },
}
