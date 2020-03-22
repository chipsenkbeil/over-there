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
    /// Prints the version of the server
    #[clap(name = "version")]
    Version(version::VersionCommand),

    /// Lists the capabilities of the server
    #[clap(name = "capabilities")]
    Capabilities(capabilities::CapabilitiesCommand),

    /// Lists the contents within the root remote directory
    #[clap(name = "ls-root-dir")]
    ListRootDir(dir::ListRootDirCommand),

    /// Lists the contents within a remote directory
    #[clap(name = "ls-dir")]
    ListDir(dir::ListDirCommand),

    /// Creates a remote directory
    #[clap(name = "mk-dir")]
    CreateDir(dir::CreateDirCommand),

    /// Moves a remote directory
    #[clap(name = "mv-dir")]
    MoveDir(dir::MoveDirCommand),

    /// Removes a remote directory
    #[clap(name = "rm-dir")]
    RemoveDir(dir::RemoveDirCommand),

    /// Writes a remote file
    #[clap(name = "write-file")]
    WriteFile(file::WriteFileCommand),

    /// Reads a remote file
    #[clap(name = "read-file")]
    ReadFile(file::ReadFileCommand),

    /// Moves a remote file
    #[clap(name = "mv-file")]
    MoveFile(file::MoveFileCommand),

    /// Removes a remote file
    #[clap(name = "rm-file")]
    RemoveFile(file::RemoveFileCommand),

    /// Executes a process remotely
    #[clap(name = "exec")]
    Exec(exec::ExecCommand),

    /// Re-attaches to a running remote process
    #[clap(name = "reattach-exec")]
    ReattachExec(exec::ReattachExecCommand),

    /// Internal debugging support against the server
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
