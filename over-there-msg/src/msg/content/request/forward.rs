use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ForwardRequest {
    ForwardRequest {
        address: SocketAddr,
        request: Box<super::Request>,
    },
}
