pub mod aead;
pub use crate::aead::{aes_gcm, aes_gcm_siv, aes_siv, AeadError, AesNonceBicrypter};

pub mod nonce;
pub use nonce::{Nonce128Bits, Nonce96Bits, NonceSize};

pub mod key;
pub use key::{Key128Bits, Key256Bits, Key512Bits, KeySize};

mod noop;
pub use noop::NoopBicrypter;

use over_there_derive::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Error)]
pub enum CryptError {
    /// Internal Error related to encryption occurred
    EncryptFailed(Box<dyn std::error::Error>),

    /// Internal Error related to decryption occurred
    DecryptFailed(Box<dyn std::error::Error>),

    /// Contains the nonce that was already used
    NonceAlreadyUsed { nonce: Vec<u8> },

    /// Contains size of nonce provided
    NonceWrongSize { provided_size: usize },

    /// When a nonce was expected and none was provided
    MissingNonce,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AssociatedData {
    None,
    Nonce96Bits(Nonce96Bits),
    Nonce128Bits(Nonce128Bits),
}

impl AssociatedData {
    pub fn to_nonce(&self) -> Option<&[u8]> {
        match self {
            Self::None => None,
            Self::Nonce96Bits(nonce) => Some(nonce),
            Self::Nonce128Bits(nonce) => Some(nonce),
        }
    }
}

/// Can both encrypt and decrypt
pub trait Bicrypter: Encrypter + Decrypter {}

/// Capable of encrypting data
pub trait Encrypter {
    fn encrypt(
        &self,
        buffer: &[u8],
        associated_data: AssociatedData,
    ) -> Result<Vec<u8>, CryptError>;
}

/// Capable of decrypting data
pub trait Decrypter {
    fn decrypt(
        &self,
        buffer: &[u8],
        associated_data: AssociatedData,
    ) -> Result<Vec<u8>, CryptError>;
}
