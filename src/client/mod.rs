pub mod capabilities;
pub mod dir;
pub mod exec;
pub mod file;
pub mod version;

use crate::parsers;
use clap::Clap;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Clap, Debug)]
pub enum Subcommand {
    #[clap(name = "version")]
    Version(version::VersionCommand),

    #[clap(name = "capabilities")]
    Capabilities(capabilities::CapabilitiesCommand),

    #[clap(name = "root-dir")]
    RootDir(dir::RootDirCommand),

    #[clap(name = "dir")]
    Dir(dir::DirCommand),

    #[clap(name = "write-file")]
    WriteFile(file::WriteFileCommand),

    #[clap(name = "read-file")]
    ReadFile(file::ReadFileCommand),

    #[clap(name = "exec")]
    Exec(exec::ExecCommand),

    #[clap(name = "reattach-exec")]
    ReattachExec(exec::ReattachExecCommand),
}

#[derive(Clap, Debug)]
/// Perform some operation as the client to some remote server instance
pub struct ClientCommand {
    #[clap(subcommand)]
    pub command: Subcommand,

    #[clap(parse(try_from_str = parsers::parse_socket_addr))]
    /// Address (<host>:<port>) of server to connect to
    pub addr: SocketAddr,

    #[clap(long, parse(try_from_str = parsers::parse_duration), default_value = "5")]
    /// Timeout (in seconds) used when communicating with the server
    pub timeout: Duration,
}
