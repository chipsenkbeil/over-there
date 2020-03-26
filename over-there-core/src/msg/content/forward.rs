use crate::msg::Msg;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ForwardArgs {
    pub address: SocketAddr,
    pub msg: Box<Msg>,
}

impl Default for ForwardArgs {
    fn default() -> Self {
        Self {
            address: SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::UNSPECIFIED,
                0,
            )),
            msg: Box::new(Msg::from(super::Content::Heartbeat)),
        }
    }
}
