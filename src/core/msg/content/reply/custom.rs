use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct CustomArgs {
    pub data: Vec<u8>,
}

impl crate::SchemaInfo for CustomArgs {}

impl From<Vec<u8>> for CustomArgs {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}
