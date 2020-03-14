use crate::{
    key::Key128Bits,
    nonce::{self, NonceSize},
    AssociatedData, Bicrypter, CryptError, Decrypter, Encrypter,
};
use aead::generic_array::GenericArray;
use aead::{Aead, NewAead};
use aes_gcm_siv::Aes128GcmSiv;

#[derive(Clone)]
pub struct Aes128GcmSivBicrypter {
    inner: Aes128GcmSiv,
    nonce_size: NonceSize,
}

/// NOTE: This is purely for derive_builder and should not be used externally
impl Default for Aes128GcmSivBicrypter {
    fn default() -> Self {
        Self::new(&crate::key::new_128bit_key())
    }
}

impl Aes128GcmSivBicrypter {
    pub fn new(key: &Key128Bits) -> Self {
        let key = GenericArray::clone_from_slice(key);
        Aes128GcmSivBicrypter {
            inner: Aes128GcmSiv::new(key),
            nonce_size: NonceSize::Nonce96Bits,
        }
    }
}

impl Bicrypter for Aes128GcmSivBicrypter {}

impl Encrypter for Aes128GcmSivBicrypter {
    fn encrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        let nonce = associated_data
            .nonce_slice()
            .ok_or(CryptError::MissingNonce)?;
        nonce::validate_nonce_size(self.nonce_size, nonce.len())?;
        self.inner
            .encrypt(GenericArray::from_slice(nonce), buffer)
            .map_err(|x| CryptError::EncryptFailed(super::make_error_string(x)))
    }

    /// Returns a new nonce to be associated when encrypting
    fn new_encrypt_associated_data(&self) -> AssociatedData {
        AssociatedData::from(self.nonce_size)
    }
}

impl Decrypter for Aes128GcmSivBicrypter {
    fn decrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        let nonce = associated_data
            .nonce_slice()
            .ok_or(CryptError::MissingNonce)?;
        nonce::validate_nonce_size(self.nonce_size, nonce.len())?;
        self.inner
            .decrypt(GenericArray::from_slice(nonce), buffer)
            .map_err(|x| CryptError::DecryptFailed(super::make_error_string(x)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key;
    use crate::nonce::{self, Nonce};
    use crate::{AssociatedData, CryptError, Decrypter, Encrypter};

    #[test]
    fn encrypt_should_fail_if_no_nonce_provided() {
        let bicrypter = Aes128GcmSivBicrypter::new(&key::new_128bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::None;

        let result = bicrypter.encrypt(&buffer, &nonce);
        match result {
            Err(CryptError::MissingNonce) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_fail_if_no_nonce_provided() {
        let bicrypter = Aes128GcmSivBicrypter::new(&key::new_128bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::None;

        let result = bicrypter.decrypt(&buffer, &nonce);
        match result {
            Err(CryptError::MissingNonce) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn encrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = Aes128GcmSivBicrypter::new(&key::new_128bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::Nonce(Nonce::Nonce128Bits(
            nonce::new_128bit_nonce(),
        ));

        let result = bicrypter.encrypt(&buffer, &nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = Aes128GcmSivBicrypter::new(&key::new_128bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::Nonce(Nonce::Nonce128Bits(
            nonce::new_128bit_nonce(),
        ));

        let result = bicrypter.decrypt(&buffer, &nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn can_encrypt_and_decrypt() {
        let bicrypter = Aes128GcmSivBicrypter::new(&key::new_128bit_key());

        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.encrypt(
            plaintext,
            &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)),
        );
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = result.unwrap();
        assert_ne!(
            result, plaintext,
            "Encryption did not alter original message"
        );

        let result = bicrypter
            .decrypt(&result, &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }
}
