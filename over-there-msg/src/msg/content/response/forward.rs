use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ForwardResponse {
    ForwardResponse {
        address: SocketAddr,
        response: Box<super::Response>,
    },
}
