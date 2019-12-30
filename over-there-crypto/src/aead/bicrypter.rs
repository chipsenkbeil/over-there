use super::{nonce, Error as AeadError};
use crate::{Bicrypter as BicrypterTrait, Decrypter, Encrypter, Error as CryptError};
use aead::{generic_array::GenericArray, Aead};
use lru::LruCache;
use std::cell::RefCell;

pub struct Bicrypter<T: Aead> {
    aead: T,
    nonce_size: nonce::Size,
    cache: Option<RefCell<LruCache<Vec<u8>, ()>>>,
}

impl<T: Aead> Bicrypter<T> {
    pub fn new(aead: T, nonce_size: nonce::Size, nonce_cache_size: usize) -> Self {
        // LruCache does not handle zero capacity itself, so we make it an
        // option where we won't do anything if it's zero
        let cache = if nonce_cache_size > 0 {
            Some(RefCell::new(LruCache::new(nonce_cache_size)))
        } else {
            None
        };

        Self {
            aead,
            nonce_size,
            cache,
        }
    }

    pub fn with_no_nonce_cache(aead: T, nonce_size: nonce::Size) -> Self {
        Self::new(aead, nonce_size, 0)
    }

    pub fn from_nonce(&self, nonce: &[u8]) -> Result<instance::CryptInstance<T>, AeadError> {
        if nonce.len() != nonce::size_to_byte_length(self.nonce_size) {
            return Err(AeadError::NonceWrongSize(nonce.len()));
        }

        if let Some(cache) = &self.cache {
            let nonce_vec = nonce.to_vec();

            if cache.borrow().contains(&nonce_vec) {
                return Err(AeadError::NonceAlreadyUsed(nonce_vec));
            }

            // Mark that we have used the nonce
            cache.borrow_mut().put(nonce_vec, ());
        }

        let nonce = GenericArray::clone_from_slice(nonce);
        Ok(instance::CryptInstance {
            aead: &self.aead,
            nonce: nonce,
        })
    }
}

impl<T: Aead> BicrypterTrait for Bicrypter<T> {}

impl<T: Aead> Encrypter for Bicrypter<T> {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
        Ok(self
            .from_nonce(&nonce::new(self.nonce_size))
            .map_err(|e| CryptError::Encrypt(Box::new(e)))?
            .encrypt(data)?)
    }
}

impl<T: Aead> Decrypter for Bicrypter<T> {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
        Ok(self
            .from_nonce(&nonce::new(self.nonce_size))
            .map_err(|e| CryptError::Decrypt(Box::new(e)))?
            .decrypt(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aead::impls;

    #[test]
    fn from_nonce_should_fail_if_nonce_already_used() {
        // TODO: Replace specific implementation used for test with
        //       stub; may require refactoring to accept general
        //       encrypt/decrypt functions?
        let aead = impls::new_aes_128_gcm(b"some 128-bit key");
        let nonce = nonce::new_96bit();

        // Make bicrypter that holds on to a single nonce
        let bicrypter = Bicrypter::new(aead, nonce::Size::Length96Bits, 1);

        assert!(bicrypter.from_nonce(&nonce).is_ok(), "First attempt failed");

        match bicrypter.from_nonce(&nonce) {
            Err(AeadError::NonceAlreadyUsed(used_nonce)) => {
                assert_eq!(nonce.to_vec(), used_nonce, "Unexpected nonce used")
            }
            Err(e) => panic!("Unexpected result: {:?}", e),
            _ => panic!("Unexpected success of reusing nonce"),
        }
    }

    #[test]
    fn from_nonce_should_succeed_if_nonce_not_used() {
        panic!("TODO: Implement");
    }

    #[test]
    fn from_nonce_should_succeed_if_nonce_no_longer_cached() {
        panic!("TODO: Implement");
    }
}

pub mod instance {
    use super::{AeadError, BicrypterTrait, CryptError, Decrypter, Encrypter};
    use aead::{generic_array::GenericArray, Aead};

    pub struct CryptInstance<'a, T: Aead> {
        pub(super) aead: &'a T,
        pub(super) nonce: GenericArray<u8, T::NonceSize>,
    }

    impl<'a, T: Aead> BicrypterTrait for CryptInstance<'a, T> {}

    impl<'a, T: Aead> Encrypter for CryptInstance<'a, T> {
        fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
            self.aead
                .encrypt(&self.nonce, data)
                .map_err(|e| CryptError::Encrypt(Box::new(AeadError::Aed(e))))
        }
    }

    impl<'a, T: Aead> Decrypter for CryptInstance<'a, T> {
        fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
            self.aead
                .decrypt(&self.nonce, data)
                .map_err(|e| CryptError::Decrypt(Box::new(AeadError::Aed(e))))
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn encrypt_should_use_underlying_nonce_with_data() {
            panic!("TODO: Implement");
        }

        #[test]
        fn decrypt_should_use_underlying_nonce_with_data() {
            panic!("TODO: Implement");
        }
    }
}
