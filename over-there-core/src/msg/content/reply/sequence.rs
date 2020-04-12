use crate::Reply;
use serde::{Deserialize, Serialize};

/// Represents arguments to a response of executing a sequence of operations
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct SequenceArgs {
    pub results: Vec<Reply>,
}
