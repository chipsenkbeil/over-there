use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ExecProcArgs {
    pub command: String,
    pub args: Vec<String>,
    pub stdin: bool,
    pub stdout: bool,
    pub stderr: bool,

    /// If provided, sets the current directory where the proc will be executed
    pub current_dir: Option<String>,
}

impl crate::SchemaInfo for ExecProcArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct WriteProcStdinArgs {
    pub id: u32,
    pub input: Vec<u8>,
}

impl crate::SchemaInfo for WriteProcStdinArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ReadProcStdoutArgs {
    pub id: u32,
}

impl crate::SchemaInfo for ReadProcStdoutArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ReadProcStderrArgs {
    pub id: u32,
}

impl crate::SchemaInfo for ReadProcStderrArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct KillProcArgs {
    pub id: u32,
}

impl crate::SchemaInfo for KillProcArgs {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct ReadProcStatusArgs {
    pub id: u32,
}

impl crate::SchemaInfo for ReadProcStatusArgs {}
