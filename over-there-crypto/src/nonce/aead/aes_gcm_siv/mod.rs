pub mod cache;

use super::AesNonceBicrypter;
use crate::key::{Key128Bits, Key256Bits};
use aead::generic_array::GenericArray;
use aead::NewAead;
use aes_gcm_siv::{Aes128GcmSiv, Aes256GcmSiv};

pub fn new_aes_128_gcm_siv_bicrypter(key: &Key128Bits) -> AesNonceBicrypter<Aes128GcmSiv> {
    let key = GenericArray::clone_from_slice(key);
    AesNonceBicrypter::new(Aes128GcmSiv::new(key))
}

pub fn new_aes_256_gcm_siv_bicrypter(key: &Key256Bits) -> AesNonceBicrypter<Aes256GcmSiv> {
    let key = GenericArray::clone_from_slice(key);
    AesNonceBicrypter::new(Aes256GcmSiv::new(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonce::{self, NonceDecrypter, NonceEncrypter};

    #[test]
    fn aes_128_gcm_siv_bicrypter_can_encrypt_and_decrypt() {
        let bicrypter = new_aes_128_gcm_siv_bicrypter(b"some 128-bit key");

        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.encrypt_with_nonce(plaintext, &nonce);
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt_with_nonce(&result.unwrap(), &nonce)
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_256_gcm_siv_bicrypter_can_encrypt_and_decrypt() {
        let bicrypter = new_aes_256_gcm_siv_bicrypter(b"some 256-bit (32-byte) key------");

        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.encrypt_with_nonce(plaintext, &nonce);
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt_with_nonce(&result.unwrap(), &nonce)
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }
}
