use super::{nonce, Error as AeadError};
use crate::{Bicrypter as BicrypterTrait, Decrypter, Encrypter, Error as CryptError};
use aead::{generic_array::GenericArray, Aead};
use lru::LruCache;
use std::cell::RefCell;

pub struct Bicrypter<T: Aead> {
    aead: T,
    cache: RefCell<LruCache<nonce::Type, ()>>,
}

impl<T: Aead> Bicrypter<T> {
    pub fn new(aead: T, nonce_cache_size: usize) -> Self {
        // NOTE: If cache size is 0, won't store any items (so disables)
        let cache = RefCell::new(LruCache::new(nonce_cache_size));

        Self { aead, cache }
    }

    pub fn with_no_nonce_cache(aead: T) -> Self {
        Self::new(aead, 0)
    }

    pub fn from_nonce(&self, nonce: &nonce::Type) -> Result<instance::CryptInstance<T>, AeadError> {
        if self.cache.borrow().contains(nonce) {
            return Err(AeadError::NonceAlreadyUsed(*nonce));
        }

        // Mark that we have used the nonce
        self.cache.borrow_mut().put(*nonce, ());

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
            .from_nonce(&nonce::new())
            .map_err(|e| CryptError::Internal(Box::new(e)))?
            .encrypt(data)?)
    }
}

impl<T: Aead> Decrypter for Bicrypter<T> {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
        Ok(self
            .from_nonce(&nonce::new())
            .map_err(|e| CryptError::Internal(Box::new(e)))?
            .decrypt(data)?)
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
                .map_err(|e| CryptError::Internal(Box::new(AeadError::Aed(e))))
        }
    }

    impl<'a, T: Aead> Decrypter for CryptInstance<'a, T> {
        fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
            self.aead
                .decrypt(&self.nonce, data)
                .map_err(|e| CryptError::Internal(Box::new(AeadError::Aed(e))))
        }
    }
}
