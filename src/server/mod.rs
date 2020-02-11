use crate::parsers;
use clap::Clap;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Clap, Debug)]
pub struct ServerCommand {
    #[clap(long = "address", parse(try_from_str = parsers::parse_socket_addr))]
    /// Address (<host>:<port>) to bind to
    pub addr: SocketAddr,

    #[clap(long, parse(try_from_str = parsers::parse_duration), default_value = "5")]
    /// Timeout (in seconds) used when communicating with clients and other servers
    pub timeout: Duration,
}
