use super::{parsers, CommonOpts};
use clap::Clap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clap, Debug)]
pub struct ServerCommand {
    #[clap(name = "address", parse(try_from_str = parsers::parse_socket_addr))]
    /// Address (<host>:<port>) to bind to
    pub addr: SocketAddr,

    #[clap(flatten)]
    pub opts: CommonOpts,

    #[clap(name = "root", default_value = &default_root())]
    pub root: PathBuf,

    #[clap(long)]
    pub no_root: bool,
}

/// Default root will be the current directory if available
fn default_root() -> String {
    env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}
