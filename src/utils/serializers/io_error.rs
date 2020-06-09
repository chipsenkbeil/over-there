use serde::{Deserialize, Serialize};
use std::io;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct SerIoError {
    description: String,
    os_code: Option<i32>,
    #[serde(
        serialize_with = "super::error_kind::serialize",
        deserialize_with = "super::error_kind::deserialize"
    )]
    error_kind: io::ErrorKind,
}

impl From<&io::Error> for SerIoError {
    fn from(x: &io::Error) -> Self {
        let error_kind = x.kind();
        let os_code = x.raw_os_error();
        let description = format!("{}", x);

        Self {
            description,
            error_kind,
            os_code,
        }
    }
}

impl From<io::Error> for SerIoError {
    fn from(x: io::Error) -> Self {
        From::from(&x)
    }
}

impl Into<io::Error> for SerIoError {
    fn into(self) -> io::Error {
        if let Some(code) = self.os_code {
            io::Error::from_raw_os_error(code)
        } else {
            io::Error::new(self.error_kind, self.description)
        }
    }
}

pub fn serialize<S>(
    error: &io::Error,
    serializer: S,
) -> serde::export::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let x: SerIoError = error.into();
    x.serialize(serializer)
}

pub fn deserialize<'de, D>(
    deserializer: D,
) -> serde::export::Result<io::Error, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let x = SerIoError::deserialize(deserializer)?;
    Ok(x.into())
}
