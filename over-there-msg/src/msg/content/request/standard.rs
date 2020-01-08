use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum StandardRequest {
    /// Make sure daemon is alive
    HeartbeatRequest,

    /// Request version
    VersionRequest,

    /// Request capabilities
    CapabilitiesRquest,
}
