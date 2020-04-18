use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct InternalDebugArgs {
    pub output: Vec<u8>,
}

impl crate::SchemaInfo for InternalDebugArgs {}
