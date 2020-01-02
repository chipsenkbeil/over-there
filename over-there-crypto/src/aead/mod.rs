pub mod aes_gcm;
pub mod aes_gcm_siv;
pub mod aes_siv;

use crate::{
    nonce::{self, NonceSize},
    AssociatedData, Bicrypter, CryptError, Decrypter, Encrypter,
};
use aead::{generic_array::GenericArray, Aead};
use over_there_derive::Error;

#[derive(Debug, Error)]
pub enum AeadError {
    /// Contains generic AED error
    Generic(aead::Error),
}

pub struct AesNonceBicrypter<T: Aead> {
    aead: T,
    nonce_size: NonceSize,
}

impl<T: Aead> AesNonceBicrypter<T> {
    pub fn new(aead: T, nonce_size: NonceSize) -> Self {
        Self { aead, nonce_size }
    }
}

impl<T: Aead> Bicrypter for AesNonceBicrypter<T> {}

impl<T: Aead> Encrypter for AesNonceBicrypter<T> {
    fn encrypt(
        &self,
        buffer: &[u8],
        associated_data: AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        let nonce = associated_data
            .nonce_slice()
            .ok_or(CryptError::MissingNonce)?;
        nonce::validate_nonce_size(self.nonce_size, nonce.len())?;
        self.aead
            .encrypt(GenericArray::from_slice(nonce), buffer)
            .map_err(|e| CryptError::EncryptFailed(Box::new(AeadError::Generic(e))))
    }

    /// Returns a new nonce to be associated when encrypting
    fn new_encrypt_associated_data(&self) -> AssociatedData {
        AssociatedData::from(self.nonce_size)
    }
}

impl<T: Aead> Decrypter for AesNonceBicrypter<T> {
    fn decrypt(
        &self,
        buffer: &[u8],
        associated_data: AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        let nonce = associated_data
            .nonce_slice()
            .ok_or(CryptError::MissingNonce)?;
        nonce::validate_nonce_size(self.nonce_size, nonce.len())?;
        self.aead
            .decrypt(GenericArray::from_slice(nonce), buffer)
            .map_err(|e| CryptError::DecryptFailed(Box::new(AeadError::Generic(e))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key;

    #[test]
    fn encrypt_should_fail_if_no_nonce_provided() {
        // NOTE: Using explicit implementation instead of mock because don't
        //       want to have to mock the Aead external type nor do I want
        //       to add extra logic inbetween to enable substituting methods
        let bicrypter: AesNonceBicrypter<_> =
            aes_gcm::new_aes_128_gcm_bicrypter(&key::new_128bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::None;

        let result = bicrypter.encrypt(&buffer, nonce);
        match result {
            Err(CryptError::MissingNonce) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_fail_if_no_nonce_provided() {
        // NOTE: Using explicit implementation instead of mock because don't
        //       want to have to mock the Aead external type nor do I want
        //       to add extra logic inbetween to enable substituting methods
        let bicrypter: AesNonceBicrypter<_> =
            aes_gcm::new_aes_128_gcm_bicrypter(&key::new_128bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::None;

        let result = bicrypter.decrypt(&buffer, nonce);
        match result {
            Err(CryptError::MissingNonce) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }
}
