use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Capability {
    /// Can do file operations
    File,

    /// Can execute programs
    Exec,

    /// Can forward msgs
    Forward,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CapabilitiesArgs {
    pub capabilities: Vec<Capability>,
}
