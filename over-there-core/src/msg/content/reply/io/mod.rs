mod fs;
mod proc;

pub use fs::*;
pub use proc::*;

use over_there_utils::serializers;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IoErrorArgs {
    pub description: String,
    pub os_code: Option<i32>,
    #[serde(
        serialize_with = "serializers::error_kind::serialize",
        deserialize_with = "serializers::error_kind::deserialize"
    )]
    pub error_kind: io::ErrorKind,
}

impl Default for IoErrorArgs {
    fn default() -> Self {
        Self {
            description: Default::default(),
            os_code: Default::default(),
            error_kind: io::ErrorKind::Other,
        }
    }
}

impl IoErrorArgs {
    pub fn invalid_file_id(id: u32) -> Self {
        Self {
            description: format!("No file open with id {}", id),
            error_kind: io::ErrorKind::InvalidInput,
            os_code: None,
        }
    }

    pub fn invalid_proc_id(id: u32) -> Self {
        Self {
            description: format!("No process executed with id {}", id),
            error_kind: io::ErrorKind::InvalidInput,
            os_code: None,
        }
    }

    pub fn pipe_unavailable() -> Self {
        Self {
            description: String::from("Resource unavailable"),
            error_kind: io::ErrorKind::BrokenPipe,
            os_code: None,
        }
    }

    pub fn from_error_with_prefix(error: io::Error, prefix: &str) -> Self {
        let mut args = Self::from(error);

        args.description = format!("{}{}", prefix, args.description);

        args
    }
}

impl From<io::Error> for IoErrorArgs {
    fn from(error: io::Error) -> Self {
        let error_kind = error.kind();
        let os_code = error.raw_os_error();
        let description = format!("{}", error);

        Self {
            description,
            error_kind,
            os_code,
        }
    }
}

impl Into<io::Error> for IoErrorArgs {
    fn into(self) -> io::Error {
        if let Some(code) = self.os_code {
            io::Error::from_raw_os_error(code)
        } else {
            io::Error::new(self.error_kind, self.description)
        }
    }
}