use crate::key::{Key256Bits, Key512Bits};
use crate::nonce::{aead::AesNonceBicrypter, cache::NonceCacheBicrypter, NonceSize};

pub fn new_aes_128_siv_cache_bicrypter(
    key: &Key256Bits,
    nonce_cache_size: usize,
) -> NonceCacheBicrypter<AesNonceBicrypter<aes_siv::Aes128SivAead>> {
    NonceCacheBicrypter::new(
        super::new_aes_128_siv_bicrypter(key),
        NonceSize::Nonce128Bits,
        nonce_cache_size,
    )
}

pub fn new_aes_256_siv_cache_bicrypter(
    key: &Key512Bits,
    nonce_cache_size: usize,
) -> NonceCacheBicrypter<AesNonceBicrypter<aes_siv::Aes256SivAead>> {
    NonceCacheBicrypter::new(
        super::new_aes_256_siv_bicrypter(key),
        NonceSize::Nonce128Bits,
        nonce_cache_size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonce;
    use crate::{AssociatedData, Decrypter, Encrypter};

    #[test]
    fn aes_128_siv_cache_bicrypter_can_encrypt_and_decrypt() {
        let encrypter = new_aes_128_siv_cache_bicrypter(b"some 256-bit (32-byte) key------", 1);
        let decrypter = new_aes_128_siv_cache_bicrypter(b"some 256-bit (32-byte) key------", 1);

        let plaintext = b"some message";
        let nonce = nonce::new_128bit_nonce();

        let result = encrypter.encrypt(plaintext, AssociatedData::Nonce128Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = decrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce128Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_256_siv_cache_bicrypter_can_encrypt_and_decrypt() {
        let key = b"some 512-bit (64-byte) key--------------------------------------";
        let encrypter = new_aes_256_siv_cache_bicrypter(key, 1);
        let decrypter = new_aes_256_siv_cache_bicrypter(key, 1);

        // Make bicrypter that holds on to a single nonce
        let plaintext = b"some message";
        let nonce = nonce::new_128bit_nonce();

        let result = encrypter.encrypt(plaintext, AssociatedData::Nonce128Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = decrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce128Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }
}
