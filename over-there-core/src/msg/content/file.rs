use over_there_utils::serializers;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoListDirContentsArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DirContentsListArgs {
    pub entries: Vec<DirEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub path: String,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoOpenFileArgs {
    pub path: String,
    pub create_if_missing: bool,
    pub write_access: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileOpenedArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoReadFileArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileContentsArgs {
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoWriteFileArgs {
    pub id: u32,
    pub sig: u32,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileWrittenArgs {
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileErrorArgs {
    pub description: String,
    pub os_code: Option<i32>,
    #[serde(
        serialize_with = "serializers::error_kind::serialize",
        deserialize_with = "serializers::error_kind::deserialize"
    )]
    pub error_kind: io::ErrorKind,
}

impl FileErrorArgs {
    pub fn invalid_file_id(id: u32) -> Self {
        Self {
            description: format!("No file open with id {}", id),
            error_kind: io::ErrorKind::InvalidInput,
            os_code: None,
        }
    }
}

impl From<io::Error> for FileErrorArgs {
    fn from(error: io::Error) -> Self {
        let error_kind = error.kind();
        let os_code = error.raw_os_error();

        // NOTE: Internally, Rust uses sys::os::error_string(code) to get a
        //       relevant message based on an Os error code; however, this is
        //       not exposed externally. Instead, the options are debug and
        //       format printing, the latter of which yields
        //       "<message> (os error <code>)" as the output
        let description = if os_code.is_some() {
            format!("{}", error)
        } else {
            use std::error::Error;
            error.description().to_string()
        };

        Self {
            description,
            error_kind,
            os_code,
        }
    }
}

impl Into<io::Error> for FileErrorArgs {
    fn into(self) -> io::Error {
        if let Some(code) = self.os_code {
            io::Error::from_raw_os_error(code)
        } else {
            io::Error::new(self.error_kind, self.description)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileSigChangedArgs {
    pub sig: u32,
}
