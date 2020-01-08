use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum StandardResponse {
    /// Report alive status
    HeartbeatResponse,

    /// Report version
    VersionResponse { version: String },

    /// Report capabilities
    CapabilitiesResponse { capabilities: Vec<String> },

    /// Generic error reponse used upon failing
    ErrorResponse { msg: String },
}
