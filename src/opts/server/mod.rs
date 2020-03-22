use super::{parsers, CommonOpts};
use clap::Clap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clap, Debug)]
pub struct ServerCommand {
    #[clap(name = "address", parse(try_from_str = parsers::parse_socket_addr))]
    /// Address (<host>:<port>) to bind to
    pub addr: SocketAddr,

    #[clap(flatten)]
    pub opts: CommonOpts,

    #[clap(long)]
    pub working_dir: Option<PathBuf>,

    #[clap(
        long, 
        parse(try_from_str = parsers::parse_duration_secs), 
        default_value = "60",
    )]
    /// Time (in seconds) between runs of the cleanup process
    pub cleanup_interval: Duration,

    #[clap(
        long, 
        parse(try_from_str = parsers::parse_duration_secs), 
        default_value = "1800",
    )]
    /// Time (in seconds) to keep file open with no activity before closing
    pub untouched_file_ttl: Duration,

    #[clap(
        long, 
        parse(try_from_str = parsers::parse_duration_secs), 
        default_value = "3600",
    )]
    /// Time (in seconds) to keep process running with no remote communication
    /// before killing
    pub untouched_proc_ttl: Duration,

    #[clap(
        long, 
        parse(try_from_str = parsers::parse_duration_secs), 
        default_value = "300",
    )]
    /// Time (in seconds) to keep dead process status available before removing
    pub dead_proc_ttl: Duration,
}
