use crate::opts::types::Authentication;
use over_there_auth::{self as auth};
use std::io;

pub enum Authenticator {
    None(auth::NoopAuthenticator),
    Sha256(auth::Sha256Authenticator),
    Sha512(auth::Sha512Authenticator),
}

impl Authenticator {
    pub fn new(
        authentication: Authentication,
        key: Option<String>,
    ) -> io::Result<Self> {
        Ok(match authentication {
            Authentication::None => Self::None(auth::NoopAuthenticator),
            Authentication::Sha256 => {
                Self::Sha256(auth::Sha256Authenticator::new(
                    key.ok_or(new_missing_key_error())?.as_bytes(),
                ))
            }
            Authentication::Sha512 => {
                Self::Sha512(auth::Sha512Authenticator::new(
                    key.ok_or(new_missing_key_error())?.as_bytes(),
                ))
            }
        })
    }
}

fn new_missing_key_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "Key is missing")
}
