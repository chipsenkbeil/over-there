use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum FileSystemResponse {
    /// List all files, directories, etc. at a path
    ListFilesResponse { paths: Vec<String> },

    /// Write the contents of a file
    WriteFileResponse { bytes_written: u32 },

    /// Read the contents of a file
    ReadFileResponse { data: Vec<u8> },
}
