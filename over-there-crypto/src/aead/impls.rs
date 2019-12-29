#[cfg(feature = "aes_gcm")]
pub fn new_aes_256_gcm(key: &[u8]) -> aes_gcm::Aes256Gcm {
    use aead::generic_array::GenericArray;
    use aead::NewAead;
    use aes_gcm::Aes256Gcm;

    let key = GenericArray::clone_from_slice(key);
    Aes256Gcm::new(key)
}

#[cfg(feature = "aes_gcm_siv")]
pub fn new_aes_256_gcm_siv(key: &[u8]) -> aes_gcm_siv::Aes256GcmSiv {
    use aead::generic_array::GenericArray;
    use aead::NewAead;
    use aes_gcm_siv::Aes256GcmSiv;

    let key = GenericArray::clone_from_slice(key);
    Aes256GcmSiv::new(key)
}

#[cfg(feature = "aes_siv")]
pub fn new_aes_256_siv(key: &[u8]) -> aes_siv::Aes256Siv {
    use aead::generic_array::GenericArray;
    use aead::NewAead;
    use aes_siv::Aes256SivAead;

    let key = GenericArray::clone_from_slice(key);
    Aes256GcmSiv::new(key)
}
