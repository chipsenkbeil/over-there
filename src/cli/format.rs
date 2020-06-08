use crate::core::Content;
use serde::Serialize;
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

pub type FormatResult = Result<String, Box<dyn std::error::Error>>;

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, EnumString, EnumVariantNames, AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum FormatOption {
    /// Human-readable format for input and output
    Human,

    /// JSON format for input and output
    Json,

    #[cfg(feature = "format-sexpression")]
    /// S-Expression format for input and output
    Sexpression,
}

/// Tries to convert provided text with the specified format to content
pub fn text_to_content(
    format_option: FormatOption,
    text: &str,
) -> Result<Content, Box<dyn std::error::Error>> {
    match format_option {
        FormatOption::Json => Ok(serde_json::from_str(text)?),

        #[cfg(feature = "format-sexpression")]
        FormatOption::Sexpression => {
            Ok(serde_lexpr::from_str(&serializable_data)?)
        }

        FormatOption::Human => Err("Cannot convert to human format".into()),
    }
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
        FormatOption::Json => serde_json::to_string(&serializable_data)?,

        #[cfg(feature = "format-sexpression")]
        FormatOption::Sexpression => {
            serde_lexpr::to_string(&serializable_data)?
        }

        FormatOption::Human => fallback(serializable_data)?,
    };

    Ok(text)
}

/// Creates a `String` using the given `format_option` and `content`,
/// falling back to the `fallback` function to render human-readable text.
pub fn format_content<F>(
    format_option: FormatOption,
    content: Content,
    fallback: F,
) -> FormatResult
where
    F: FnOnce(Content) -> FormatResult,
{
    format(format_option, content, fallback)
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

/// Formats `content` using the given `format_option`, falling back
/// to the `fallback` function to render human-readable text, and prints to
/// stdout with a newline.
pub fn format_content_println<F>(
    format_option: FormatOption,
    content: Content,
    fallback: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(Content) -> FormatResult,
{
    format_println(format_option, content, fallback)
}
