use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct CreateDirArgs {
    pub path: String,
    pub include_components: bool,
}

impl crate::SchemaInfo for CreateDirArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct RenameDirArgs {
    pub from: String,
    pub to: String,
}

impl crate::SchemaInfo for RenameDirArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct RemoveDirArgs {
    pub path: String,
    pub non_empty: bool,
}

impl crate::SchemaInfo for RemoveDirArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ListDirContentsArgs {
    pub path: String,
}

impl crate::SchemaInfo for ListDirContentsArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct OpenFileArgs {
    pub path: String,
    pub create_if_missing: bool,
    pub write_access: bool,
    pub read_access: bool,
}

impl crate::SchemaInfo for OpenFileArgs {}

impl From<String> for OpenFileArgs {
    fn from(path: String) -> Self {
        Self {
            path,
            create_if_missing: true,
            write_access: true,
            read_access: true,
        }
    }
}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct CloseFileArgs {
    pub id: u32,
    pub sig: u32,
}

impl crate::SchemaInfo for CloseFileArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct RenameUnopenedFileArgs {
    pub from: String,
    pub to: String,
}

impl crate::SchemaInfo for RenameUnopenedFileArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct RenameFileArgs {
    pub id: u32,
    pub sig: u32,
    pub to: String,
}

impl crate::SchemaInfo for RenameFileArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct RemoveUnopenedFileArgs {
    pub path: String,
}

impl crate::SchemaInfo for RemoveUnopenedFileArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct RemoveFileArgs {
    pub id: u32,
    pub sig: u32,
}

impl crate::SchemaInfo for RemoveFileArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ReadFileArgs {
    pub id: u32,
    pub sig: u32,
}

impl crate::SchemaInfo for ReadFileArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct WriteFileArgs {
    pub id: u32,
    pub sig: u32,
    pub contents: Vec<u8>,
}

impl crate::SchemaInfo for WriteFileArgs {}
