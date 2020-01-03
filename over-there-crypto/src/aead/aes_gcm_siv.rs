use super::AesNonceBicrypter;
use crate::key::{Key128Bits, Key256Bits};
use crate::nonce::NonceSize;
use aead::generic_array::GenericArray;
use aead::NewAead;
use aes_gcm_siv::{Aes128GcmSiv, Aes256GcmSiv};

pub fn new_aes_128_gcm_siv_bicrypter(key: &Key128Bits) -> AesNonceBicrypter<Aes128GcmSiv> {
    let key = GenericArray::clone_from_slice(key);
    AesNonceBicrypter::new(Aes128GcmSiv::new(key), NonceSize::Nonce96Bits)
}

pub fn new_aes_256_gcm_siv_bicrypter(key: &Key256Bits) -> AesNonceBicrypter<Aes256GcmSiv> {
    let key = GenericArray::clone_from_slice(key);
    AesNonceBicrypter::new(Aes256GcmSiv::new(key), NonceSize::Nonce96Bits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key;
    use crate::nonce::{self, Nonce};
    use crate::{AssociatedData, CryptError, Decrypter, Encrypter};

    #[test]
    fn aes_128_gcm_siv_encrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = new_aes_128_gcm_siv_bicrypter(&key::new_128bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::Nonce(Nonce::Nonce128Bits(nonce::new_128bit_nonce()));

        let result = bicrypter.encrypt(&buffer, &nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn aes_128_gcm_siv_decrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = new_aes_128_gcm_siv_bicrypter(&key::new_128bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::Nonce(Nonce::Nonce128Bits(nonce::new_128bit_nonce()));

        let result = bicrypter.decrypt(&buffer, &nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn aes_256_gcm_siv_encrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = new_aes_256_gcm_siv_bicrypter(&key::new_256bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::Nonce(Nonce::Nonce128Bits(nonce::new_128bit_nonce()));

        let result = bicrypter.encrypt(&buffer, &nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn aes_256_gcm_siv_decrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = new_aes_256_gcm_siv_bicrypter(&key::new_256bit_key());
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::Nonce(Nonce::Nonce128Bits(nonce::new_128bit_nonce()));

        let result = bicrypter.decrypt(&buffer, &nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn aes_128_gcm_siv_bicrypter_can_encrypt_and_decrypt() {
        let bicrypter = new_aes_128_gcm_siv_bicrypter(b"some 128-bit key");

        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result =
            bicrypter.encrypt(plaintext, &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt(
                &result.unwrap(),
                &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)),
            )
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_256_gcm_siv_bicrypter_can_encrypt_and_decrypt() {
        let bicrypter = new_aes_256_gcm_siv_bicrypter(b"some 256-bit (32-byte) key------");

        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result =
            bicrypter.encrypt(plaintext, &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt(
                &result.unwrap(),
                &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)),
            )
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }
}
