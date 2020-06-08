use super::Request;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Represents arguments to a request for a batch of operations
#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct BatchArgs {
    pub operations: Vec<Request>,
}

impl crate::core::SchemaInfo for BatchArgs {}

impl From<Vec<Request>> for BatchArgs {
    fn from(operations: Vec<Request>) -> Self {
        Self { operations }
    }
}
