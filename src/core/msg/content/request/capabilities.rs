use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    JsonSchema, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq,
)]
pub struct CapabilitiesArgs;

impl crate::core::SchemaInfo for CapabilitiesArgs {}
