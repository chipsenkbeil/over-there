use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DirCreatedArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DirRenamedArgs {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DirRemovedArgs {
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
pub struct FileOpenedArgs {
    pub id: u32,
    pub sig: u32,
    pub path: String,
    pub read: bool,
    pub write: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileClosedArgs {
    pub id: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct UnopenedFileRenamedArgs {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileRenamedArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct UnopenedFileRemovedArgs {
    pub path: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileRemovedArgs {
    pub id: u32,
    pub sig: u32,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct FileContentsArgs {
    pub id: u32,
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