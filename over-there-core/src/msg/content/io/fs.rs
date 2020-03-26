use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoCreateDirArgs {
    pub path: String,
    pub include_components: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DirCreatedArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoRenameDirArgs {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DirRenamedArgs {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoRemoveDirArgs {
    pub path: String,
    pub non_empty: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DirRemovedArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoListDirContentsArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DirContentsListArgs {
    pub path: String,
    pub entries: Vec<DirEntry>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub path: String,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileOpenedArgs {
    pub id: u32,
    pub sig: u32,
    pub path: String,
    pub read: bool,
    pub write: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoCloseFileArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileClosedArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoRenameUnopenedFileArgs {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct UnopenedFileRenamedArgs {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoRenameFileArgs {
    pub id: u32,
    pub sig: u32,
    pub to: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileRenamedArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoRemoveUnopenedFileArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct UnopenedFileRemovedArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoRemoveFileArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileRemovedArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoReadFileArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileContentsArgs {
    pub id: u32,
    pub contents: Vec<u8>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoWriteFileArgs {
    pub id: u32,
    pub sig: u32,
    pub contents: Vec<u8>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileWrittenArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileSigChangedArgs {
    pub id: u32,
    pub sig: u32,
}
