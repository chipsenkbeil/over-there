use super::Content;
use serde::{Deserialize, Serialize};

/// Represents arguments to a request for a batch of operations
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct DoBatchArgs {
    pub operations: Vec<Content>,
}
