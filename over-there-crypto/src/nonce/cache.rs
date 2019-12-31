use super::{NonceBicrypter, NonceSize};
use crate::{AssociatedData, Bicrypter, CryptError, Decrypter, Encrypter};
use lru::LruCache;
use std::cell::RefCell;

pub struct NonceCacheBicrypter<T: NonceBicrypter> {
    nonce_bicrypter: T,
    nonce_size: NonceSize,
    cache: Option<RefCell<LruCache<Vec<u8>, ()>>>,
}

impl<T: NonceBicrypter> NonceCacheBicrypter<T> {
    pub fn new(nonce_bicrypter: T, nonce_size: NonceSize, nonce_cache_size: usize) -> Self {
        // LruCache does not handle zero capacity itself, so we make it an
        // option where we won't do anything if it's zero
        let cache = if nonce_cache_size > 0 {
            Some(RefCell::new(LruCache::new(nonce_cache_size)))
        } else {
            None
        };

        Self {
            nonce_bicrypter,
            nonce_size,
            cache,
        }
    }

    pub fn with_no_nonce_cache(nonce_bicrypter: T, nonce_size: NonceSize) -> Self {
        Self::new(nonce_bicrypter, nonce_size, 0)
    }

    fn register_nonce(&self, nonce: &[u8]) -> Result<Vec<u8>, CryptError> {
        if nonce.len() != super::nonce_size_to_byte_length(self.nonce_size) {
            return Err(CryptError::NonceWrongSize {
                provided_size: nonce.len(),
            });
        }

        if let Some(cache) = &self.cache {
            let nonce_vec = nonce.to_vec();
            if cache.borrow().contains(&nonce_vec) {
                return Err(CryptError::NonceAlreadyUsed { nonce: nonce_vec });
            }

            // Mark that we have used the nonce
            cache.borrow_mut().put(nonce_vec, ());
        }

        Ok(nonce.to_vec())
    }
}

impl<T: NonceBicrypter> Bicrypter for NonceCacheBicrypter<T> {}

impl<T: NonceBicrypter> Encrypter for NonceCacheBicrypter<T> {
    fn encrypt(
        &self,
        buffer: &[u8],
        associated_data: AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        let nonce = associated_data
            .to_nonce()
            .map(|n| self.register_nonce(&n))
            .transpose()?
            .ok_or(CryptError::MissingNonce)?;
        self.nonce_bicrypter.encrypt_with_nonce(buffer, &nonce)
    }
}

impl<T: NonceBicrypter> Decrypter for NonceCacheBicrypter<T> {
    fn decrypt(
        &self,
        buffer: &[u8],
        associated_data: AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        let nonce = associated_data
            .to_nonce()
            .map(|n| self.register_nonce(&n))
            .transpose()?
            .ok_or(CryptError::MissingNonce)?;
        self.nonce_bicrypter.decrypt_with_nonce(buffer, &nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonce::{self, NonceDecrypter, NonceEncrypter};
    use crate::CryptError;

    struct StubNonceBicrypter(fn(&[u8], &[u8]) -> Result<Vec<u8>, CryptError>);
    impl NonceBicrypter for StubNonceBicrypter {}
    impl NonceEncrypter for StubNonceBicrypter {
        fn encrypt_with_nonce(&self, buffer: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptError> {
            (self.0)(buffer, nonce)
        }
    }
    impl NonceDecrypter for StubNonceBicrypter {
        fn decrypt_with_nonce(&self, buffer: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptError> {
            (self.0)(buffer, nonce)
        }
    }

    #[test]
    fn encrypt_should_fail_if_no_nonce_provided() {
        let bicrypter = NonceCacheBicrypter::with_no_nonce_cache(
            StubNonceBicrypter(|_, _| Ok(vec![])),
            NonceSize::Nonce96Bits,
        );
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::None;

        let result = bicrypter.encrypt(&buffer, nonce);
        match result {
            Err(CryptError::MissingNonce) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn encrypt_should_fail_if_nonce_is_wrong_size() {
        let bicrypter = NonceCacheBicrypter::with_no_nonce_cache(
            StubNonceBicrypter(|_, _| Ok(vec![])),
            NonceSize::Nonce96Bits,
        );
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::Nonce128Bits(nonce::new_128bit_nonce());

        let result = bicrypter.encrypt(&buffer, nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn encrypt_should_fail_if_caching_nonce_and_nonce_already_used() {
        let bicrypter = NonceCacheBicrypter::new(
            StubNonceBicrypter(|_, _| Ok(vec![])),
            NonceSize::Nonce96Bits,
            1,
        );
        let buffer = vec![1, 2, 3];
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.encrypt(&buffer, AssociatedData::Nonce96Bits(nonce));
        assert!(
            result.is_ok(),
            "First encrypt unexpectedly failed: {:?}",
            result
        );

        let result = bicrypter.encrypt(&buffer, AssociatedData::Nonce96Bits(nonce));
        match result {
            Err(CryptError::NonceAlreadyUsed { nonce: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn encrypt_should_fail_if_underlying_encrypt_fails() {
        let bicrypter = NonceCacheBicrypter::new(
            StubNonceBicrypter(|_, _| {
                Err(CryptError::EncryptFailed(Box::new(std::io::Error::from(
                    std::io::ErrorKind::Other,
                ))))
            }),
            NonceSize::Nonce96Bits,
            1,
        );
        let buffer = vec![1, 2, 3];

        let nonce = AssociatedData::Nonce96Bits(nonce::new_96bit_nonce());
        let result = bicrypter.encrypt(&buffer, nonce);
        match result {
            Err(CryptError::EncryptFailed(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn encrypt_should_succeed_if_can_encrypt_buffer() {
        let bicrypter = NonceCacheBicrypter::new(
            StubNonceBicrypter(|_, _| Ok(vec![])),
            NonceSize::Nonce96Bits,
            1,
        );
        let buffer = vec![1, 2, 3];

        let nonce = AssociatedData::Nonce96Bits(nonce::new_96bit_nonce());
        let result = bicrypter.encrypt(&buffer, nonce);
        assert!(
            result.is_ok(),
            "First encrypt unexpectedly failed: {:?}",
            result
        );

        let nonce = AssociatedData::Nonce96Bits(nonce::new_96bit_nonce());
        let result = bicrypter.encrypt(&buffer, nonce);
        assert!(
            result.is_ok(),
            "Second encrypt unexpectedly failed: {:?}",
            result
        );
    }

    #[test]
    fn decrypt_should_fail_if_no_nonce_provided() {
        let bicrypter = NonceCacheBicrypter::with_no_nonce_cache(
            StubNonceBicrypter(|_, _| Ok(vec![])),
            NonceSize::Nonce96Bits,
        );
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::None;

        let result = bicrypter.decrypt(&buffer, nonce);
        match result {
            Err(CryptError::MissingNonce) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_fail_if_nonce_is_wrong_size() {
        let bicrypter = NonceCacheBicrypter::with_no_nonce_cache(
            StubNonceBicrypter(|_, _| Ok(vec![])),
            NonceSize::Nonce96Bits,
        );
        let buffer = vec![1, 2, 3];
        let nonce = AssociatedData::Nonce128Bits(nonce::new_128bit_nonce());

        let result = bicrypter.decrypt(&buffer, nonce);
        match result {
            Err(CryptError::NonceWrongSize { provided_size: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_fail_if_caching_nonce_and_nonce_already_used() {
        let bicrypter = NonceCacheBicrypter::new(
            StubNonceBicrypter(|_, _| Ok(vec![])),
            NonceSize::Nonce96Bits,
            1,
        );
        let buffer = vec![1, 2, 3];
        let nonce = nonce::new_96bit_nonce();

        let result = bicrypter.decrypt(&buffer, AssociatedData::Nonce96Bits(nonce));
        assert!(
            result.is_ok(),
            "First encrypt unexpectedly failed: {:?}",
            result
        );

        let result = bicrypter.decrypt(&buffer, AssociatedData::Nonce96Bits(nonce));
        match result {
            Err(CryptError::NonceAlreadyUsed { nonce: _ }) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_fail_if_underlying_decrypt_fails() {
        let bicrypter = NonceCacheBicrypter::new(
            StubNonceBicrypter(|_, _| {
                Err(CryptError::DecryptFailed(Box::new(std::io::Error::from(
                    std::io::ErrorKind::Other,
                ))))
            }),
            NonceSize::Nonce96Bits,
            1,
        );
        let buffer = vec![1, 2, 3];

        let nonce = AssociatedData::Nonce96Bits(nonce::new_96bit_nonce());
        let result = bicrypter.decrypt(&buffer, nonce);
        match result {
            Err(CryptError::DecryptFailed(_)) => (),
            x => panic!("Unexpected result: {:?}", x),
        }
    }

    #[test]
    fn decrypt_should_succeed_if_can_decrypt_buffer() {
        let bicrypter = NonceCacheBicrypter::new(
            StubNonceBicrypter(|_, _| Ok(vec![])),
            NonceSize::Nonce96Bits,
            1,
        );
        let buffer = vec![1, 2, 3];

        let nonce = AssociatedData::Nonce96Bits(nonce::new_96bit_nonce());
        let result = bicrypter.decrypt(&buffer, nonce);
        assert!(
            result.is_ok(),
            "First encrypt unexpectedly failed: {:?}",
            result
        );

        let nonce = AssociatedData::Nonce96Bits(nonce::new_96bit_nonce());
        let result = bicrypter.decrypt(&buffer, nonce);
        assert!(
            result.is_ok(),
            "Second encrypt unexpectedly failed: {:?}",
            result
        );
    }
}
