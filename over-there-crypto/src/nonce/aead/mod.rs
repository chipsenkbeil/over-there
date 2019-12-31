pub mod aes_gcm;
pub mod aes_gcm_siv;
pub mod aes_siv;

use super::{CryptError, NonceBicrypter, NonceDecrypter, NonceEncrypter};
use aead::{generic_array::GenericArray, Aead};

#[derive(Debug)]
pub enum Error {
    /// Contains generic AED error
    Aed(aead::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::Aed(error) => write!(f, "AED: {:?}", error),
        }
    }
}

pub struct AesNonceBicrypter<T: Aead> {
    aead: T,
}

impl<T: Aead> AesNonceBicrypter<T> {
    pub fn new(aead: T) -> Self {
        Self { aead }
    }
}

impl<T: Aead> NonceBicrypter for AesNonceBicrypter<T> {}

impl<T: Aead> NonceEncrypter for AesNonceBicrypter<T> {
    fn encrypt_with_nonce(&self, buffer: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptError> {
        let nonce = GenericArray::from_slice(nonce);
        self.aead
            .encrypt(nonce, buffer)
            .map_err(|e| CryptError::EncryptFailed(Box::new(Error::Aed(e))))
    }
}

impl<T: Aead> NonceDecrypter for AesNonceBicrypter<T> {
    fn decrypt_with_nonce(&self, buffer: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptError> {
        let nonce = GenericArray::from_slice(nonce);
        self.aead
            .decrypt(nonce, buffer)
            .map_err(|e| CryptError::EncryptFailed(Box::new(Error::Aed(e))))
    }
}
