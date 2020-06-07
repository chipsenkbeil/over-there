use crate::format::FormatOption;
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
    )]
    pub format: FormatOption,

    /// If provided, will maintain an interactive session where multiple
    /// raw inputs can be provided over time, only concluding if the program
    /// is terminated or stdin is closed
    #[clap(short, long)]
    pub interactive: bool,
}
