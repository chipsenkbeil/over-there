use crate::cli::format::FormatOption;
use clap::Clap;
use strum::VariantNames;

/// Performs an operation using raw input as the instruction, only
/// valid for non-Human input such as JSON
#[derive(Clap, Debug)]
pub struct RawCommand {
    /// Raw input to be sent directly to the server
    pub input: Option<String>,

    /// Specifies the format of input to the server and output from the server
    #[clap(
        short, 
        long, 
        parse(try_from_str), 
        possible_values = &FormatOption::VARIANTS.iter()
            .filter(|v| **v != FormatOption::Human.as_ref())
            .map(|v| *v)
            .collect::<Vec<&str>>(), 
        default_value = FormatOption::Json.as_ref(),
    )]
    pub format: FormatOption,

    /// If provided, will maintain an interactive session where multiple
    /// raw inputs can be provided over time, only concluding if the program
    /// is terminated or stdin is closed
    #[clap(short, long)]
    pub interactive: bool,

    /// If provided, will send and receive content and meta information.
    /// Useful for specifying additional information when sending content and
    /// having that information available on replies (such as callback IDs)
    #[clap(short, long)]
    pub meta_mode: bool,
}
