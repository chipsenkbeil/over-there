use super::Request;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(JsonSchema, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ForwardArgs {
    pub address: SocketAddr,
    pub request: Box<Request>,
}

impl crate::SchemaInfo for ForwardArgs {}
