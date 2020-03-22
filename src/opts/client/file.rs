use clap::Clap;

#[derive(Clap, Debug)]
/// Writes a file on the server
pub struct WriteFileCommand {
    #[clap(parse(try_from_str))]
    /// Path to the file
    pub path: String,

    #[clap(parse(try_from_str))]
    /// Content to write to the file
    pub contents: String,
}

#[derive(Clap, Debug)]
/// Reads a file on the server
pub struct ReadFileCommand {
    #[clap(parse(try_from_str))]
    /// Path to the file
    pub path: String,
}

#[derive(Clap, Debug)]
/// Moves a file at the specified path on the server to the new path
pub struct MoveFileCommand {
    #[clap(parse(try_from_str))]
    /// Origin path of the file to move
    pub from: String,

    #[clap(parse(try_from_str))]
    /// Destination path of the file to move
    pub to: String,
}

#[derive(Clap, Debug)]
/// Removes a file at the specified path on the server
pub struct RemoveFileCommand {
    #[clap(parse(try_from_str))]
    /// Path of the file to remove
    pub path: String,
}
