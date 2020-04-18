use super::LazilyTransformedRequest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Represents arguments to a request for a sequence of operations
#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct SequenceArgs {
    pub operations: Vec<LazilyTransformedRequest>,
}

impl crate::SchemaInfo for SequenceArgs {}

impl From<Vec<LazilyTransformedRequest>> for SequenceArgs {
    fn from(operations: Vec<LazilyTransformedRequest>) -> Self {
        Self { operations }
    }
}
