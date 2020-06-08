use crate::cli::opts::types::Encryption;
use over_there_crypto::{self as crypto, key::Key};
use std::io;

macro_rules! match_key_or_err {
    ($key_enum:path, $key_str:expr) => {{
        let empty_str = String::new();
        if let Some($key_enum(key)) =
            Key::from_slice($key_str.as_ref().unwrap_or(&empty_str).as_bytes())
        {
            Ok(key)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Provided encryption key not right length",
            ))
        }
    }};
}

pub enum Bicrypter {
    None(crypto::NoopBicrypter),
    Aes128Gcm(crypto::Aes128GcmBicrypter),
    Aes256Gcm(crypto::Aes256GcmBicrypter),
    Aes128GcmSiv(crypto::Aes128GcmSivBicrypter),
    Aes256GcmSiv(crypto::Aes256GcmSivBicrypter),
    Aes128Siv(crypto::Aes128SivBicrypter),
    Aes256Siv(crypto::Aes256SivBicrypter),
}

impl Bicrypter {
    pub fn new(encyption: Encryption, key: Option<String>) -> io::Result<Self> {
        Ok(match encyption {
            Encryption::None => Self::None(crypto::NoopBicrypter),
            Encryption::Aes128Gcm => {
                Self::Aes128Gcm(crypto::Aes128GcmBicrypter::new(
                    &match_key_or_err!(Key::Key128Bits, key)?,
                ))
            }
            Encryption::Aes256Gcm => {
                Self::Aes256Gcm(crypto::Aes256GcmBicrypter::new(
                    &match_key_or_err!(Key::Key256Bits, key)?,
                ))
            }
            Encryption::Aes128GcmSiv => {
                Self::Aes128GcmSiv(crypto::Aes128GcmSivBicrypter::new(
                    &match_key_or_err!(Key::Key128Bits, key)?,
                ))
            }
            Encryption::Aes256GcmSiv => {
                Self::Aes256GcmSiv(crypto::Aes256GcmSivBicrypter::new(
                    &match_key_or_err!(Key::Key256Bits, key)?,
                ))
            }
            Encryption::Aes128Siv => {
                Self::Aes128Siv(crypto::Aes128SivBicrypter::new(
                    &match_key_or_err!(Key::Key256Bits, key)?,
                ))
            }
            Encryption::Aes256Siv => {
                Self::Aes256Siv(crypto::Aes256SivBicrypter::new(
                    &match_key_or_err!(Key::Key512Bits, key)?,
                ))
            }
        })
    }
}
