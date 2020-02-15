use super::AssociatedData;
use crate::{Bicrypter, CryptError, Decrypter, Encrypter};
use lru::LruCache;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct NonceCacheBicrypter<T: Bicrypter> {
    bicrypter: T,
    cache: Option<Arc<RwLock<LruCache<Vec<u8>, ()>>>>,
}

impl<T: Bicrypter> NonceCacheBicrypter<T> {
    pub fn new(bicrypter: T, nonce_cache_size: usize) -> Self {
        // LruCache does not handle zero capacity itself, so we make it an
        // option where we won't do anything if it's zero
        let cache = if nonce_cache_size > 0 {
            Some(Arc::new(RwLock::new(LruCache::new(nonce_cache_size))))
        } else {
            None
        };

        Self { bicrypter, cache }
    }

    pub fn with_no_nonce_cache(bicrypter: T) -> Self {
        Self::new(bicrypter, 0)
    }

    fn register_nonce<'a>(&self, nonce: &'a [u8]) -> Result<&'a [u8], CryptError> {
        if let Some(cache) = &self.cache {
            let nonce_vec = nonce.to_vec();
            if cache.read().unwrap().contains(&nonce_vec) {
                return Err(CryptError::NonceAlreadyUsed { nonce: nonce_vec });
            }

            // Mark that we have used the nonce
            cache.write().unwrap().put(nonce_vec, ());
        }

        Ok(nonce)
    }
}

impl<T: Bicrypter> Bicrypter for NonceCacheBicrypter<T> {}

impl<T: Bicrypter> Encrypter for NonceCacheBicrypter<T> {
    fn encrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        // Register the nonce if provided, and then pass on to the underlying
        // encrypter
        if let Some(nonce) = associated_data.nonce_slice() {
            self.register_nonce(nonce)?;
        }
        self.bicrypter.encrypt(buffer, associated_data)
    }

    /// Returns underlying bicrypter's associated data
    fn new_encrypt_associated_data(&self) -> AssociatedData {
        self.bicrypter.new_encrypt_associated_data()
    }
}

impl<T: Bicrypter> Decrypter for NonceCacheBicrypter<T> {
    fn decrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        // Register the nonce if provided, and then pass on to the underlying
        // decrypter
        if let Some(nonce) = associated_data.nonce_slice() {
            self.register_nonce(nonce)?;
        }
        self.bicrypter.decrypt(buffer, associated_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonce::{self, Nonce};
    use crate::{AssociatedData, CryptError};

    #[derive(Clone)]
    struct StubBicrypter(fn(&[u8], &AssociatedData) -> Result<Vec<u8>, CryptError>);
    impl Bicrypter for StubBicrypter {}
    impl Encrypter for StubBicrypter {
        fn encrypt(
            &self,
            buffer: &[u8],
            associated_data: &AssociatedData,
        ) -> Result<Vec<u8>, CryptError> {
            (self.0)(buffer, associated_data)
        }

        fn new_encrypt_associated_data(&self) -> AssociatedData {
            AssociatedData::None
        }
    }
    impl Decrypter for StubBicrypter {
        fn decrypt(
            &self,
            buffer: &[u8],
            associated_data: &AssociatedData,
        ) -> Result<Vec<u8>, CryptError> {
            (self.0)(buffer, associated_data)
        }
    }

    #[test]
    fn encrypt_should_fail_if_caching_nonce_and_nonce_already_used() {
        let bicrypter = NonceCacheBicrypter::new(StubBicrypter(|_, _| Ok(vec![])), 1);
        let buffer = vec![1, 2, 3];
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.encrypt(&buffer, &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)));
        assert!(
            result.is_ok(),
            "First encrypt unexpectedly failed: {:?}",
            result
        );

        let result = bicrypter.encrypt(&buffer, &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)));
        match result {
            Err(CryptError::NonceAlreadyUsed { nonce: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn encrypt_should_fail_if_underlying_encrypt_fails() {
        let bicrypter = NonceCacheBicrypter::new(
            StubBicrypter(|_, _| Err(CryptError::EncryptFailed(From::from("Some error")))),
            1,
        );
        let buffer = vec![1, 2, 3];

        let nonce = AssociatedData::Nonce(Nonce::Nonce96Bits(nonce::new_96bit_nonce()));
        let result = bicrypter.encrypt(&buffer, &nonce);
        match result {
            Err(CryptError::EncryptFailed(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn encrypt_should_succeed_if_can_encrypt_buffer() {
        let bicrypter = NonceCacheBicrypter::new(StubBicrypter(|_, _| Ok(vec![])), 1);
        let buffer = vec![1, 2, 3];

        let nonce = AssociatedData::Nonce(Nonce::Nonce96Bits(nonce::new_96bit_nonce()));
        let result = bicrypter.encrypt(&buffer, &nonce);
        assert!(
            result.is_ok(),
            "First encrypt unexpectedly failed: {:?}",
            result
        );

        let nonce = AssociatedData::Nonce(Nonce::Nonce96Bits(nonce::new_96bit_nonce()));
        let result = bicrypter.encrypt(&buffer, &nonce);
        assert!(
            result.is_ok(),
            "Second encrypt unexpectedly failed: {:?}",
            result
        );
    }

    #[test]
    fn decrypt_should_fail_if_caching_nonce_and_nonce_already_used() {
        let bicrypter = NonceCacheBicrypter::new(StubBicrypter(|_, _| Ok(vec![])), 1);
        let buffer = vec![1, 2, 3];
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.decrypt(&buffer, &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)));
        assert!(
            result.is_ok(),
            "First encrypt unexpectedly failed: {:?}",
            result
        );

        let result = bicrypter.decrypt(&buffer, &AssociatedData::Nonce(Nonce::Nonce96Bits(nonce)));
        match result {
            Err(CryptError::NonceAlreadyUsed { nonce: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_fail_if_underlying_decrypt_fails() {
        let bicrypter = NonceCacheBicrypter::new(
            StubBicrypter(|_, _| Err(CryptError::DecryptFailed(From::from("Some error")))),
            1,
        );
        let buffer = vec![1, 2, 3];

        let nonce = AssociatedData::Nonce(Nonce::Nonce96Bits(nonce::new_96bit_nonce()));
        let result = bicrypter.decrypt(&buffer, &nonce);
        match result {
            Err(CryptError::DecryptFailed(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_succeed_if_can_decrypt_buffer() {
        let bicrypter = NonceCacheBicrypter::new(StubBicrypter(|_, _| Ok(vec![])), 1);
        let buffer = vec![1, 2, 3];

        let nonce = AssociatedData::Nonce(Nonce::Nonce96Bits(nonce::new_96bit_nonce()));
        let result = bicrypter.decrypt(&buffer, &nonce);
        assert!(
            result.is_ok(),
            "First encrypt unexpectedly failed: {:?}",
            result
        );

        let nonce = AssociatedData::Nonce(Nonce::Nonce96Bits(nonce::new_96bit_nonce()));
        let result = bicrypter.decrypt(&buffer, &nonce);
        assert!(
            result.is_ok(),
            "Second encrypt unexpectedly failed: {:?}",
            result
        );
    }
}
