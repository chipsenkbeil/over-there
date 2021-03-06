use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Capability {
    /// Can send custom binary blobs
    Custom,

    /// Can do file operations
    FileSystem,

    /// Can execute programs
    Exec,

    /// Can forward msgs
    Forward,
}

impl crate::core::SchemaInfo for Capability {}

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct CapabilitiesArgs {
    pub capabilities: Vec<Capability>,
}

impl crate::core::SchemaInfo for CapabilitiesArgs {}
