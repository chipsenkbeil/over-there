use serde::Serialize;
use strum_macros::{EnumString, EnumVariantNames};

pub type FormatResult = Result<String, Box<dyn std::error::Error>>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumString, EnumVariantNames)]
pub enum FormatOption {
    /// Human-readable format for input and output
    Human,

    #[cfg(feature = "format-json")]
    /// JSON format for input and output
    Json,

    #[cfg(feature = "format-sexpression")]
    /// S-Expression format for input and output
    Sexpression,
}

/// Creates a `String` using the given `format_option` and `serializable_data`,
/// falling back to the `fallback` function to render human-readable text.
pub fn format<T, F>(
    format_option: FormatOption,
    serializable_data: T,
    fallback: F,
) -> FormatResult
where
    T: Serialize,
    F: FnOnce(T) -> FormatResult,
{
    let text = match format_option {
        #[cfg(feature = "format-json")]
        FormatOption::Json => serde_json::to_string(&serializable_data)?,

        #[cfg(feature = "format-sexpression")]
        FormatOption::Sexpression => {
            serde_lexpr::to_string(&serializable_data)?
        }

        FormatOption::Human => fallback(serializable_data)?,
    };

    Ok(text)
}

/// Formats `serializeable_data` using the given `format_option`, falling back
/// to the `fallback` function to render human-readable text, and prints to
/// stdout with a newline.
pub fn format_println<T, F>(
    format_option: FormatOption,
    serializeable_data: T,
    fallback: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    T: Serialize,
    F: FnOnce(T) -> FormatResult,
{
    let text = format(format_option, serializeable_data, fallback)?;

    println!("{}", text);

    Ok(())
}
