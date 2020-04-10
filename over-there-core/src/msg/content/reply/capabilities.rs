use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct CapabilitiesArgs {
    pub capabilities: Vec<Capability>,
}
