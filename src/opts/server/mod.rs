use super::{parsers, CommonOpts};
use clap::Clap;
use std::net::SocketAddr;

#[derive(Clap, Debug)]
pub struct ServerCommand {
    #[clap(name = "address", parse(try_from_str = parsers::parse_socket_addr))]
    /// Address (<host>:<port>) to bind to
    pub addr: SocketAddr,

    #[clap(flatten)]
    pub opts: CommonOpts,
}
