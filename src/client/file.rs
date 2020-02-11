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
