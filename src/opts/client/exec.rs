use clap::Clap;

#[derive(Clap, Debug)]
/// Executes a process on the server
pub struct ExecCommand {
    #[clap(parse(try_from_str))]
    /// The command to execute
    pub command: String,

    #[clap(parse(try_from_str))]
    /// The arguments for the command
    pub args: Vec<String>,

    #[clap(short, long)]
    /// Whether or not to send stdin from this process to the remote process
    pub stdin: bool,

    #[clap(short, long)]
    /// Whether or not to detach the client from the remote process, thereby
    /// not terminating the process if the client disconnects
    pub detached: bool,
}

#[derive(Clap, Debug)]
/// Reattaches to a running program on the server
pub struct ReattachExecCommand {
    #[clap(parse(try_from_str))]
    /// The id of the remote process to connect to
    pub id: u32,

    #[clap(short, long)]
    /// Whether or not to send stdin from this process to the remote process
    pub stdin: bool,
}
