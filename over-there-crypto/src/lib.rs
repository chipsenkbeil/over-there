pub mod aead;

pub mod nonce;
pub use nonce::{Nonce128Bits, Nonce96Bits, NonceSize};

pub mod noop;

mod error;
pub use error::Error as CryptError;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum AssociatedData {
    None,
    Nonce96Bits(Nonce96Bits),
    Nonce128Bits(Nonce128Bits),
}

impl AssociatedData {
    pub fn to_nonce(&self) -> Option<Vec<u8>> {
        match self {
            Self::None => None,
            Self::Nonce96Bits(nonce) => Some(nonce.to_vec()),
            Self::Nonce128Bits(nonce) => Some(nonce.to_vec()),
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
