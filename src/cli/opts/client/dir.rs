use clap::Clap;

/// List files and directories at the root of the server
#[derive(Clap, Debug)]
pub struct ListRootDirCommand {}

/// List files and directories at the specified path
#[derive(Clap, Debug)]
pub struct ListDirCommand {
    /// Path to the directory whose contents to list
    #[clap(parse(try_from_str))]
    pub path: String,
}

/// Creates a directory at the specified path on the server
#[derive(Clap, Debug)]
pub struct CreateDirCommand {
    /// Path to the directory to create
    #[clap(parse(try_from_str))]
    pub path: String,

    /// If provided, will make parent directories as needed
    #[clap(short, long)]
    pub parents: bool,
}

/// Moves a directory at the specified path on the server to the new path
#[derive(Clap, Debug)]
pub struct MoveDirCommand {
    /// Origin path of the directory to move
    #[clap(parse(try_from_str))]
    pub from: String,

    /// Destination path of the directory to move
    #[clap(parse(try_from_str))]
    pub to: String,
}

/// Removes a directory at the specified path on the server
#[derive(Clap, Debug)]
pub struct RemoveDirCommand {
    /// Path of the directory to remove
    #[clap(parse(try_from_str))]
    pub path: String,

    /// If provided, will remove directory even if not empty
    #[clap(long)]
    pub non_empty: bool,
}
