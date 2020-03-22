pub mod capabilities;
pub mod dir;
pub mod exec;
pub mod file;
pub mod internal_debug;
pub mod version;

use super::CommonOpts;
use clap::Clap;

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

    #[clap(name = "internal-debug")]
    InternalDebug(internal_debug::InternalDebugCommand),
}

#[derive(Clap, Debug)]
/// Perform some operation as the client to some remote server instance
pub struct ClientCommand {
    #[clap(subcommand)]
    pub command: Subcommand,

    /// Address (<host>:<port>) of server to connect to
    pub addr: String,

    /// If provided, will attempt to resolve the address of a server as IPv6
    /// instead of IPv4 in the event that both are yielded from a DNS resolution
    #[clap(short = "6", long)]
    pub ipv6: bool,

    #[clap(flatten)]
    pub opts: CommonOpts,
}
