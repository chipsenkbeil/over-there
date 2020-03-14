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

    #[clap(name = "root")]
    pub root: Option<PathBuf>,

    #[clap(long)]
    pub no_root: bool,
}

impl ServerCommand {
    /// Returns the root if set, or a default root if available
    pub fn root_or_default(&self) -> PathBuf {
        match &self.root {
            Some(root) => root.to_path_buf(),
            None => env::current_dir().ok().unwrap_or_default(),
        }
    }
}
