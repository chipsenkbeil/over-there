use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoListDirContentsArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DirContentsListArgs {
    pub entries: Vec<DirEntry>,
}

impl From<Vec<DirEntry>> for DirContentsListArgs {
    fn from(entries: Vec<DirEntry>) -> Self {
        Self { entries }
    }
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
    pub read_access: bool,
}

impl From<String> for DoOpenFileArgs {
    fn from(path: String) -> Self {
        Self {
            path,
            create_if_missing: true,
            write_access: true,
            read_access: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileOpenedArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoCloseFileArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileClosedArgs {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoRenameFileArgs {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileRenamedArgs {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DoRemoveFileArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileRemovedArgs {}

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
pub struct FileSigChangedArgs {
    pub sig: u32,
}
