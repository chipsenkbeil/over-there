use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ProcStartedArgs {
    pub id: u32,
}

impl crate::SchemaInfo for ProcStartedArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ProcStdinWrittenArgs {
    pub id: u32,
}

impl crate::SchemaInfo for ProcStdinWrittenArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ProcStdoutContentsArgs {
    pub id: u32,
    pub output: Vec<u8>,
}

impl crate::SchemaInfo for ProcStdoutContentsArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ProcStderrContentsArgs {
    pub id: u32,
    pub output: Vec<u8>,
}

impl crate::SchemaInfo for ProcStderrContentsArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ProcKilledArgs {
    pub id: u32,
    pub exit_code: Option<i32>,
}

impl crate::SchemaInfo for ProcKilledArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ProcStatusArgs {
    pub id: u32,
    pub is_alive: bool,
    pub exit_code: Option<i32>,
}

impl crate::SchemaInfo for ProcStatusArgs {}
