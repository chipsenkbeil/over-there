pub mod capabilities;
pub mod dir;
pub mod exec;
pub mod file;
pub mod internal_debug;
pub mod raw;
pub mod version;

use super::CommonOpts;
use crate::cli::format::FormatOption;
use clap::Clap;
use std::path::PathBuf;
use strum::VariantNames;

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
    #[clap(name = "reattach")]
    ReattachExec(exec::ReattachExecCommand),

    /// Performs an operation using raw input as the instruction, only
    /// valid for non-Human input such as JSON
    #[clap(name = "raw")]
    Raw(raw::RawCommand),

    /// Internal debugging support against the server
    #[clap(name = "internal-debug")]
    InternalDebug(internal_debug::InternalDebugCommand),
}

/// Perform some operation as the client to some remote server instance
#[derive(Clap, Debug)]
pub struct ClientCommand {
    #[clap(subcommand)]
    pub command: Subcommand,

    /// Address (<host>:<port>) of server to connect to
    pub addr: String,

    /// If provided, will attempt to resolve the address of a server as IPv6
    /// instead of IPv4 in the event that both are yielded from a DNS resolution
    #[clap(short = "6", long)]
    pub ipv6: bool,

    /// Specifies the format of output from the client
    #[clap(
        short, 
        long, 
        parse(try_from_str), 
        possible_values = &FormatOption::VARIANTS,
        default_value = FormatOption::Human.as_ref(),
    )]
    pub output_format: FormatOption,

    /// If provided, will print out exit information when an exec process
    /// exits (or is killed) if using human-readable format; all other
    /// formats will always yield a status output
    #[clap(long)]
    pub exit_print: bool,

    /// If provided, will redirect stdout as a result of an operation to the
    /// file specified by the provided path
    #[clap(long)]
    pub redirect_stdout: Option<PathBuf>,

    /// If provided, will redirect stderr as a result of an operation to the
    /// file specified by the provided path
    #[clap(long)]
    pub redirect_stderr: Option<PathBuf>,

    #[clap(flatten)]
    pub opts: CommonOpts,
}
