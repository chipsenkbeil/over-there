pub mod aes_gcm_siv;

use super::error::Error as CryptError;
use super::{Bicrypter, Decrypter, Encrypter};
use aead::{generic_array::GenericArray, Aead};

/// Represents a 96-bit nonce (12 bytes)
pub type Nonce = [u8; 12];

#[derive(Debug)]
pub enum Error {
    Aed(aead::Error),
    NonceAlreadyUsed(Nonce),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Aed(error) => write!(f, "AED: {:?}", error),
            Error::NonceAlreadyUsed(nonce) => write!(f, "Nonce already used: {:?}", nonce),
        }
    }
}

pub struct CryptInstance<'a, T: Aead> {
    aead: &'a T,
    nonce: GenericArray<u8, T::NonceSize>,
}

impl<'a, T: Aead> Bicrypter for CryptInstance<'a, T> {}

impl<'a, T: Aead> Encrypter for CryptInstance<'a, T> {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
        self.aead
            .encrypt(&self.nonce, data)
            .map_err(|e| CryptError::Internal(Box::new(Error::Aed(e))))
    }
}

impl<'a, T: Aead> Decrypter for CryptInstance<'a, T> {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
        self.aead
            .decrypt(&self.nonce, data)
            .map_err(|e| CryptError::Internal(Box::new(Error::Aed(e))))
    }
}
