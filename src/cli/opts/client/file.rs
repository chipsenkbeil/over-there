use clap::Clap;

/// Writes a file on the server
#[derive(Clap, Debug)]
pub struct WriteFileCommand {
    /// Path to the file
    #[clap(parse(try_from_str))]
    pub path: String,

    /// Content to write to the file
    #[clap(parse(try_from_str))]
    pub contents: String,
}

/// Reads a file on the server
#[derive(Clap, Debug)]
pub struct ReadFileCommand {
    /// Path to the file
    #[clap(parse(try_from_str))]
    pub path: String,
}

/// Moves a file at the specified path on the server to the new path
#[derive(Clap, Debug)]
pub struct MoveFileCommand {
    /// Origin path of the file to move
    #[clap(parse(try_from_str))]
    pub from: String,

    /// Destination path of the file to move
    #[clap(parse(try_from_str))]
    pub to: String,
}

/// Removes a file at the specified path on the server
#[derive(Clap, Debug)]
pub struct RemoveFileCommand {
    /// Path of the file to remove
    #[clap(parse(try_from_str))]
    pub path: String,
}
