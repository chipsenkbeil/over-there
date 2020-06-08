use crate::core::Reply;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Represents arguments to a response of executing a sequence of operations
#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct SequenceArgs {
    pub results: Vec<Reply>,
}

impl crate::core::SchemaInfo for SequenceArgs {}

impl From<Vec<Reply>> for SequenceArgs {
    fn from(results: Vec<Reply>) -> Self {
        Self { results }
    }
}
