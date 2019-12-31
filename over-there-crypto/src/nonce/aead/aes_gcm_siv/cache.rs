use crate::key::{Key128Bits, Key256Bits};
use crate::nonce::{aead::AesNonceBicrypter, cache::NonceCacheBicrypter, NonceSize};

pub fn new_aes_128_gcm_siv_cache_bicrypter(
    key: &Key128Bits,
    nonce_cache_size: usize,
) -> NonceCacheBicrypter<AesNonceBicrypter<aes_gcm_siv::Aes128GcmSiv>> {
    NonceCacheBicrypter::new(
        super::new_aes_128_gcm_siv_bicrypter(key),
        NonceSize::Nonce96Bits,
        nonce_cache_size,
    )
}

pub fn new_aes_256_gcm_siv_cache_bicrypter(
    key: &Key256Bits,
    nonce_cache_size: usize,
) -> NonceCacheBicrypter<AesNonceBicrypter<aes_gcm_siv::Aes256GcmSiv>> {
    NonceCacheBicrypter::new(
        super::new_aes_256_gcm_siv_bicrypter(key),
        NonceSize::Nonce96Bits,
        nonce_cache_size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonce;
    use crate::{AssociatedData, Decrypter, Encrypter};

    #[test]
    fn aes_128_gcm_siv_cache_bicrypter_can_encrypt_and_decrypt() {
        let encrypter = new_aes_128_gcm_siv_cache_bicrypter(b"some 128-bit key", 1);
        let decrypter = new_aes_128_gcm_siv_cache_bicrypter(b"some 128-bit key", 1);

        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result = encrypter.encrypt(plaintext, AssociatedData::Nonce96Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = decrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce96Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_256_gcm_siv_cache_bicrypter_can_encrypt_and_decrypt() {
        let encrypter = new_aes_256_gcm_siv_cache_bicrypter(b"some 256-bit (32-byte) key------", 1);
        let decrypter = new_aes_256_gcm_siv_cache_bicrypter(b"some 256-bit (32-byte) key------", 1);

        // Make bicrypter that holds on to a single nonce
        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result = encrypter.encrypt(plaintext, AssociatedData::Nonce96Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = decrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce96Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }
}
