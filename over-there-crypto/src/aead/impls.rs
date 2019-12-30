pub fn new_aes_128_gcm(key: &[u8; 16]) -> aes_gcm::Aes128Gcm {
    use aead::generic_array::GenericArray;
    use aead::NewAead;
    use aes_gcm::Aes128Gcm;

    let key = GenericArray::clone_from_slice(key);
    Aes128Gcm::new(key)
}

pub fn new_aes_256_gcm(key: &[u8; 32]) -> aes_gcm::Aes256Gcm {
    use aead::generic_array::GenericArray;
    use aead::NewAead;
    use aes_gcm::Aes256Gcm;

    let key = GenericArray::clone_from_slice(key);
    Aes256Gcm::new(key)
}

pub fn new_aes_128_gcm_siv(key: &[u8; 16]) -> aes_gcm_siv::Aes128GcmSiv {
    use aead::generic_array::GenericArray;
    use aead::NewAead;
    use aes_gcm_siv::Aes128GcmSiv;

    let key = GenericArray::clone_from_slice(key);
    Aes128GcmSiv::new(key)
}

pub fn new_aes_256_gcm_siv(key: &[u8; 32]) -> aes_gcm_siv::Aes256GcmSiv {
    use aead::generic_array::GenericArray;
    use aead::NewAead;
    use aes_gcm_siv::Aes256GcmSiv;

    let key = GenericArray::clone_from_slice(key);
    Aes256GcmSiv::new(key)
}

pub fn new_aes_128_siv(key: &[u8; 32]) -> aes_siv::Aes128SivAead {
    use aead::generic_array::GenericArray;
    use aead::NewAead;
    use aes_siv::Aes128SivAead;

    // NOTE: Key needs to be 256-bit (32-byte); the
    //       number here is 128-bit security with a
    //       256-bit key
    let key = GenericArray::clone_from_slice(key);
    Aes128SivAead::new(key)
}

pub fn new_aes_256_siv(key: &[u8; 64]) -> aes_siv::Aes256SivAead {
    use aead::generic_array::GenericArray;
    use aead::NewAead;
    use aes_siv::Aes256SivAead;

    // NOTE: Key needs to be 512-bit (64-byte); the
    //       number here is 256-bit security with a
    //       512-bit key
    let key = GenericArray::clone_from_slice(key);
    Aes256SivAead::new(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aead::bicrypter::Bicrypter;
    use crate::nonce::{self, NonceSize};
    use crate::{AssociatedData, Decrypter, Encrypter};

    #[test]
    fn aes_128_gcm_can_encrypt_and_decrypt() {
        let aead = new_aes_128_gcm(b"some 128-bit key");

        // Make bicrypter that holds on to a single nonce
        let bicrypter = Bicrypter::with_no_nonce_cache(aead, NonceSize::Nonce96Bits);
        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.encrypt(plaintext, AssociatedData::Nonce96Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce96Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_256_gcm_can_encrypt_and_decrypt() {
        let aead = new_aes_256_gcm(b"some 256-bit (32-byte) key------");

        // Make bicrypter that holds on to a single nonce
        let bicrypter = Bicrypter::with_no_nonce_cache(aead, NonceSize::Nonce96Bits);
        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.encrypt(plaintext, AssociatedData::Nonce96Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce96Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_128_gcm_siv_can_encrypt_and_decrypt() {
        let aead = new_aes_128_gcm_siv(b"some 128-bit key");

        // Make bicrypter that holds on to a single nonce
        let bicrypter = Bicrypter::with_no_nonce_cache(aead, NonceSize::Nonce96Bits);
        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.encrypt(plaintext, AssociatedData::Nonce96Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce96Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_256_gcm_siv_can_encrypt_and_decrypt() {
        let aead = new_aes_256_gcm_siv(b"some 256-bit (32-byte) key------");

        // Make bicrypter that holds on to a single nonce
        let bicrypter = Bicrypter::with_no_nonce_cache(aead, NonceSize::Nonce96Bits);
        let plaintext = b"some message";
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.encrypt(plaintext, AssociatedData::Nonce96Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce96Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_128_siv_can_encrypt_and_decrypt() {
        let aead = new_aes_128_siv(b"some 256-bit (32-byte) key------");

        // Make bicrypter that holds on to a single nonce
        let bicrypter = Bicrypter::with_no_nonce_cache(aead, NonceSize::Nonce128Bits);
        let plaintext = b"some message";
        let nonce = nonce::new_128bit_nonce();

        let result = bicrypter.encrypt(plaintext, AssociatedData::Nonce128Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce128Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }

    #[test]
    fn aes_256_siv_can_encrypt_and_decrypt() {
        let key = b"some 512-bit (64-byte) key------some 512-bit (64-byte) key------";
        let aead = new_aes_256_siv(key);

        // Make bicrypter that holds on to a single nonce
        let bicrypter = Bicrypter::with_no_nonce_cache(aead, NonceSize::Nonce128Bits);
        let plaintext = b"some message";
        let nonce = nonce::new_128bit_nonce();

        let result = bicrypter.encrypt(plaintext, AssociatedData::Nonce128Bits(nonce));
        assert!(result.is_ok(), "Failed to encrypt: {:?}", result);

        let result = bicrypter
            .decrypt(&result.unwrap(), AssociatedData::Nonce128Bits(nonce))
            .expect("Failed to decrypt");
        assert_eq!(result, plaintext, "Decrypted data is wrong: {:?}", result);
    }
}
