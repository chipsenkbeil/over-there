use clap::Clap;

#[derive(Clap, Debug)]
/// List files and directories at the root of the server
pub struct ListRootDirCommand {}

#[derive(Clap, Debug)]
/// List files and directories at the specified path
pub struct ListDirCommand {
    #[clap(parse(try_from_str))]
    /// Path to the directory whose contents to list
    pub path: String,
}

#[derive(Clap, Debug)]
/// Creates a directory at the specified path on the server
pub struct CreateDirCommand {
    #[clap(parse(try_from_str))]
    /// Path to the directory to create
    pub path: String,

    #[clap(short, long)]
    /// If provided, will make parent directories as needed
    pub parents: bool,
}

#[derive(Clap, Debug)]
/// Moves a directory at the specified path on the server to the new path
pub struct MoveDirCommand {
    #[clap(parse(try_from_str))]
    /// Origin path of the directory to move
    pub from: String,

    #[clap(parse(try_from_str))]
    /// Destination path of the directory to move
    pub to: String,
}

#[derive(Clap, Debug)]
/// Removes a directory at the specified path on the server
pub struct RemoveDirCommand {
    #[clap(parse(try_from_str))]
    /// Path of the directory to remove
    pub path: String,

    #[clap(long)]
    /// If provided, will remove directory even if not empty
    pub non_empty: bool,
}
