use crate::{Content, LazilyTransformedContent};
use serde::{Deserialize, Serialize};

/// Represents arguments to a request for a sequence of operations
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct SequenceArgs {
    pub operations: Vec<LazilyTransformedContent>,
}
