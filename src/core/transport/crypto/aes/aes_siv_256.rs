use super::super::{
    key::{self, Key512Bits},
    nonce::{self, NonceSize},
    AssociatedData, Bicrypter, CryptError, Decrypter, Encrypter,
};
use aead::generic_array::GenericArray;
use aead::{Aead, NewAead};
use aes_siv::Aes256SivAead;

pub struct Aes256SivBicrypter {
    inner: Aes256SivAead,
    nonce_size: NonceSize,
}

/// NOTE: This is purely for derive_builder and should not be used externally
impl Default for Aes256SivBicrypter {
    fn default() -> Self {
        Self::new(&key::new_512bit_key())
    }
}

impl Aes256SivBicrypter {
    pub fn new(key: &Key512Bits) -> Self {
        let key = GenericArray::clone_from_slice(key);
        Aes256SivBicrypter {
            inner: Aes256SivAead::new(key),
            nonce_size: NonceSize::Nonce128Bits,
        }
    }
}

impl Bicrypter for Aes256SivBicrypter {}

impl Encrypter for Aes256SivBicrypter {
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

impl Decrypter for Aes256SivBicrypter {
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
    use nonce::Nonce;

    #[test]
    fn encrypt_should_fail_if_no_nonce_provided() {
        let bicrypter = Aes256SivBicrypter::new(&key::new_512bit_key());
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
        let bicrypter = Aes256SivBicrypter::new(&key::new_512bit_key());
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
        // Uses 128-bit nonce
        let bicrypter = Aes256SivBicrypter::new(&key::new_512bit_key());
        let buffer = vec![1, 2, 3];
        let nonce =
            AssociatedData::Nonce(Nonce::Nonce96Bits(nonce::new_96bit_nonce()));

        let result = bicrypter.encrypt(&buffer, &nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 128-bit nonce
        let bicrypter = Aes256SivBicrypter::new(&key::new_512bit_key());
        let buffer = vec![1, 2, 3];
        let nonce =
            AssociatedData::Nonce(Nonce::Nonce96Bits(nonce::new_96bit_nonce()));

        let result = bicrypter.decrypt(&buffer, &nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn can_encrypt_and_decrypt() {
        let bicrypter = Aes256SivBicrypter::new(&key::new_512bit_key());

        let plaintext = b"some message";
        let nonce = nonce::new_128bit_nonce();

        let result = bicrypter.encrypt(
            plaintext,
            &AssociatedData::Nonce(Nonce::Nonce128Bits(nonce)),
        );
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = result.unwrap();
        assert_ne!(
            result, plaintext,
            "Encryption did not alter original message"
        );

        let result = bicrypter
            .decrypt(
                &result,
                &AssociatedData::Nonce(Nonce::Nonce128Bits(nonce)),
            )
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }
}
