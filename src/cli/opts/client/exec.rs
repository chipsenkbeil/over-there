use crate::cli::opts::parsers;
use clap::Clap;
use std::time::Duration;

/// Executes a process on the server
#[derive(Clap, Debug)]
pub struct ExecCommand {
    /// The command to execute
    #[clap(parse(try_from_str))]
    pub command: String,

    /// The arguments for the command
    #[clap(parse(try_from_str))]
    pub args: Vec<String>,

    /// Whether or not to send stdin from this process to the remote process
    #[clap(long)]
    pub no_stdin: bool,

    /// Whether or not to detach the client from the remote process, thereby
    /// not terminating the process if the client disconnects
    #[clap(short, long)]
    pub detached: bool,

    /// If provided, changes the current working directory for the new process
    #[clap(long)]
    pub current_dir: Option<String>,

    /// The time (in milliseconds) to wait after a process exits (or is killed)
    /// to receive lingering stdout/stderr before closing the remote connection
    #[clap(
        long,
        parse(try_from_str = parsers::parse_duration_millis),
        default_value = "100"
    )]
    pub post_exit_duration: Duration,
}

/// Reattaches to a running program on the server
#[derive(Clap, Debug)]
pub struct ReattachExecCommand {
    /// The id of the remote process to connect to
    #[clap(parse(try_from_str))]
    pub id: u32,

    /// Whether or not to send stdin from this process to the remote process
    #[clap(long)]
    pub no_stdin: bool,

    /// The time (in milliseconds) to wait after a process exits (or is killed)
    /// to receive lingering stdout/stderr before closing the remote connection
    #[clap(
        long,
        parse(try_from_str = parsers::parse_duration_millis),
        default_value = "100"
    )]
    pub post_exit_duration: Duration,
}
