pub mod aes;
pub use aes::{
    Aes128GcmBicrypter, Aes128GcmSivBicrypter, Aes128SivBicrypter,
    Aes256GcmBicrypter, Aes256GcmSivBicrypter, Aes256SivBicrypter, AesError,
};

pub mod nonce;
pub use nonce::{Nonce, Nonce128Bits, Nonce96Bits, NonceSize};

pub mod key;
pub use key::{Key128Bits, Key256Bits, Key512Bits, KeySize};

mod noop;
pub use noop::NoopBicrypter;

mod closure;
pub use closure::{ClosureDecrypter, ClosureEncrypter};

pub mod split;

use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Display, Error)]
pub enum CryptError {
    /// Internal Error related to encryption occurred
    EncryptFailed(#[error(ignore)] String),

    /// Internal Error related to decryption occurred
    DecryptFailed(#[error(ignore)] String),

    /// Contains the nonce that was already used
    #[display(fmt = "nonce:{:?}", nonce)]
    NonceAlreadyUsed { nonce: Vec<u8> },

    /// Contains size of nonce provided
    NonceWrongSize { provided_size: usize },

    /// When a nonce was expected and none was provided
    MissingNonce,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AssociatedData {
    None,
    Nonce(Nonce),
}

impl AssociatedData {
    pub fn nonce(&self) -> Option<&Nonce> {
        match self {
            AssociatedData::None => None,
            AssociatedData::Nonce(nonce) => Some(nonce),
        }
    }

    pub fn nonce_slice(&self) -> Option<&[u8]> {
        self.nonce().map(|n| n.as_slice())
    }
}

impl From<Nonce> for AssociatedData {
    fn from(nonce: Nonce) -> Self {
        Self::Nonce(nonce)
    }
}

impl From<Option<Nonce>> for AssociatedData {
    fn from(nonce: Option<Nonce>) -> Self {
        match nonce {
            None => Self::None,
            Some(nonce) => Self::Nonce(nonce),
        }
    }
}

impl From<NonceSize> for AssociatedData {
    fn from(nonce_size: NonceSize) -> Self {
        Self::from(Nonce::from(nonce_size))
    }
}

/// Can both encrypt and decrypt
pub trait Bicrypter: Encrypter + Decrypter {}

/// Capable of encrypting data
pub trait Encrypter {
    fn encrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError>;

    /// Encrypter generates its own associated data, useful for producing
    /// a new nonce, etc.
    fn new_encrypt_associated_data(&self) -> AssociatedData;
}

/// Capable of decrypting data
pub trait Decrypter {
    fn decrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError>;
}
