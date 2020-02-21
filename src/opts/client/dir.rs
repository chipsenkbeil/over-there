use clap::Clap;

#[derive(Clap, Debug)]
/// List files and directories at the root of the server
pub struct RootDirCommand {}

#[derive(Clap, Debug)]
/// List files and directories at the specified path
pub struct DirCommand {
    #[clap(parse(try_from_str))]
    /// Path to the directory whose contents to list
    pub path: String,
}
