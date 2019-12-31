pub mod cache;

use super::AesNonceBicrypter;
use crate::key::{Key256Bits, Key512Bits};
use aead::generic_array::GenericArray;
use aead::NewAead;
use aes_siv::{Aes128SivAead, Aes256SivAead};

pub fn new_aes_128_siv_bicrypter(key: &Key256Bits) -> AesNonceBicrypter<Aes128SivAead> {
    // NOTE: Key needs to be 256-bit (32-byte); the
    //       number here is 128-bit security with a
    //       256-bit key
    let key = GenericArray::clone_from_slice(key);
    AesNonceBicrypter::new(Aes128SivAead::new(key))
}

pub fn new_aes_256_siv_bicrypter(key: &Key512Bits) -> AesNonceBicrypter<Aes256SivAead> {
    // NOTE: Key needs to be 512-bit (64-byte); the
    //       number here is 256-bit security with a
    //       512-bit key
    let key = GenericArray::clone_from_slice(key);
    AesNonceBicrypter::new(Aes256SivAead::new(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonce::{self, NonceDecrypter, NonceEncrypter};

    #[test]
    fn aes_128_siv_bicrypter_can_encrypt_and_decrypt() {
        let bicrypter = new_aes_128_siv_bicrypter(b"some 256-bit (32-byte) key------");

        // Make bicrypter that holds on to a single nonce
        let plaintext = b"some message";
        let nonce = nonce::new_128bit_nonce();

        let result = bicrypter.encrypt_with_nonce(plaintext, &nonce);
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt_with_nonce(&result.unwrap(), &nonce)
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_256_siv_bicrypter_can_encrypt_and_decrypt() {
        let key = b"some 512-bit (64-byte) key------some 512-bit (64-byte) key------";
        let bicrypter = new_aes_256_siv_bicrypter(key);

        // Make bicrypter that holds on to a single nonce
        let plaintext = b"some message";
        let nonce = nonce::new_128bit_nonce();

        let result = bicrypter.encrypt_with_nonce(plaintext, &nonce);
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt_with_nonce(&result.unwrap(), &nonce)
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }
}
