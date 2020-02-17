use super::AesNonceBicrypter;
use crate::key::{Key128Bits, Key256Bits};
use crate::nonce::NonceSize;
use aead::generic_array::GenericArray;
use aead::NewAead;
use aes_gcm::{Aes128Gcm, Aes256Gcm};

pub fn new_aes_128_gcm_bicrypter(
    key: &Key128Bits,
) -> AesNonceBicrypter<Aes128Gcm> {
    let key = GenericArray::clone_from_slice(key);
    AesNonceBicrypter::new(Aes128Gcm::new(key), NonceSize::Nonce96Bits)
}

pub fn new_aes_256_gcm_bicrypter(
    key: &Key256Bits,
) -> AesNonceBicrypter<Aes256Gcm> {
    let key = GenericArray::clone_from_slice(key);
    AesNonceBicrypter::new(Aes256Gcm::new(key), NonceSize::Nonce96Bits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key;
    use crate::nonce::{self, Nonce};
    use crate::{AssociatedData, CryptError, Decrypter, Encrypter};

    #[test]
    fn aes_128_gcm_encrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = new_aes_128_gcm_bicrypter(&key::new_128bit_key());
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
    fn aes_128_gcm_decrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = new_aes_128_gcm_bicrypter(&key::new_128bit_key());
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
    fn aes_256_gcm_encrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = new_aes_256_gcm_bicrypter(&key::new_256bit_key());
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
    fn aes_256_gcm_decrypt_should_fail_if_nonce_is_wrong_size() {
        // Uses 96-bit nonce
        let bicrypter = new_aes_256_gcm_bicrypter(&key::new_256bit_key());
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
    fn aes_128_gcm_bicrypter_can_encrypt_and_decrypt() {
        let bicrypter = new_aes_128_gcm_bicrypter(b"some 128-bit key");

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

    #[test]
    fn aes_256_gcm_bicrypter_can_encrypt_and_decrypt() {
        let bicrypter =
            new_aes_256_gcm_bicrypter(b"some 256-bit (32-byte) key------");

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
