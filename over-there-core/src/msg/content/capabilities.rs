use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CapabilitiesArgs {
    pub capabilities: Vec<Capability>,
}
