use crate::core::Reply;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct BatchArgs {
    pub results: Vec<Reply>,
}

impl crate::core::SchemaInfo for BatchArgs {}

impl From<Vec<Reply>> for BatchArgs {
    fn from(results: Vec<Reply>) -> Self {
        Self { results }
    }
}
