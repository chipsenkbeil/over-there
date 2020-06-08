use super::{parsers, CommonOpts};
use clap::Clap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Binding to a given address and listen for requests
#[derive(Clap, Debug)]
pub struct ServerCommand {
    /// Address (<host>:<port>) to bind to
    #[clap(name = "address", parse(try_from_str = parsers::parse_socket_addr))]
    pub addr: SocketAddr,

    #[clap(flatten)]
    pub opts: CommonOpts,

    /// If provided, changes the current working directory of the server
    #[clap(long)]
    pub working_dir: Option<PathBuf>,

    /// Time (in seconds) between runs of the cleanup process
    #[clap(
        long, 
        parse(try_from_str = parsers::parse_duration_secs), 
        default_value = "60",
    )]
    pub cleanup_interval: Duration,

    /// Minimum time (in seconds) to keep file open with no activity before
    /// closing
    #[clap(
        long, 
        parse(try_from_str = parsers::parse_duration_secs), 
        default_value = "150",
    )]
    pub untouched_file_ttl: Duration,

    /// Minimum time (in seconds) to keep process running with no remote
    /// communication before killing
    #[clap(
        long, 
        parse(try_from_str = parsers::parse_duration_secs), 
        default_value = "300",
    )]
    pub untouched_proc_ttl: Duration,

    /// Minimum time (in seconds) to keep dead process status available before
    /// removing
    #[clap(
        long, 
        parse(try_from_str = parsers::parse_duration_secs), 
        default_value = "30",
    )]
    pub dead_proc_ttl: Duration,
}
