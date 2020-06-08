use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct DirCreatedArgs {
    pub path: String,
}

impl crate::SchemaInfo for DirCreatedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct DirRenamedArgs {
    pub from: String,
    pub to: String,
}

impl crate::SchemaInfo for DirRenamedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct DirRemovedArgs {
    pub path: String,
}

impl crate::SchemaInfo for DirRemovedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct DirContentsListArgs {
    pub path: String,
    pub entries: Vec<DirEntry>,
}

impl crate::SchemaInfo for DirContentsListArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct DirEntry {
    pub path: String,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
}

impl crate::SchemaInfo for DirEntry {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct FileOpenedArgs {
    pub id: u32,
    pub sig: u32,
    pub path: String,
    pub read: bool,
    pub write: bool,
}

impl crate::SchemaInfo for FileOpenedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct FileClosedArgs {
    pub id: u32,
}

impl crate::SchemaInfo for FileClosedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct UnopenedFileRenamedArgs {
    pub from: String,
    pub to: String,
}

impl crate::SchemaInfo for UnopenedFileRenamedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct FileRenamedArgs {
    pub id: u32,
    pub sig: u32,
}

impl crate::SchemaInfo for FileRenamedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct UnopenedFileRemovedArgs {
    pub path: String,
}

impl crate::SchemaInfo for UnopenedFileRemovedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct FileRemovedArgs {
    pub id: u32,
    pub sig: u32,
}

impl crate::SchemaInfo for FileRemovedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct FileContentsArgs {
    pub id: u32,
    pub contents: Vec<u8>,
}

impl crate::SchemaInfo for FileContentsArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct FileWrittenArgs {
    pub id: u32,
    pub sig: u32,
}

impl crate::SchemaInfo for FileWrittenArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct FileSigChangedArgs {
    pub id: u32,
    pub sig: u32,
}

impl crate::SchemaInfo for FileSigChangedArgs {}

impl ToString for FileSigChangedArgs {
    fn to_string(&self) -> String {
        format!("File {} signature changed", self.id)
    }
}
