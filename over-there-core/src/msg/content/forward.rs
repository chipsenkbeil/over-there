use crate::msg::Msg;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ForwardArgs {
    pub address: SocketAddr,
    pub msg: Box<Msg>,
}
