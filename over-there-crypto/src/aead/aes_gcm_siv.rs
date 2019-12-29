use aead::{self, generic_array::GenericArray, NewAead};
use aes_gcm_siv::Aes256GcmSiv;
use lru::LruCache;
use std::cell::RefCell;

use super::{CryptInstance, Error, Nonce};
use crate::{error::Error as CryptError, Bicrypter, Decrypter, Encrypter};

pub struct Producer {
    aead: Aes256GcmSiv,
    cache: RefCell<LruCache<Nonce, ()>>,
}

impl Producer {
    pub fn new(key: &[u8], nonce_cache_size: usize) -> Self {
        let key = GenericArray::clone_from_slice(key);
        let aead = Aes256GcmSiv::new(key);

        // NOTE: If cache size is 0, won't store any items (so disables)
        let cache = RefCell::new(LruCache::new(nonce_cache_size));

        Self { aead, cache }
    }

    pub fn with_no_nonce_cache(key: &[u8]) -> Self {
        Self::new(key, 0)
    }

    pub fn from_nonce(&self, nonce: &Nonce) -> Result<CryptInstance<Aes256GcmSiv>, Error> {
        if self.cache.borrow().contains(nonce) {
            return Err(Error::NonceAlreadyUsed(*nonce));
        }

        // Mark that we
        self.cache.borrow_mut().put(*nonce, ());

        let nonce = GenericArray::from_slice(nonce);
        Ok(CryptInstance {
            aead: &self.aead,
            nonce: *nonce,
        })
    }

    /// Produces a 96-bit nonce (12 bytes)
    fn make_nonce() -> Nonce {
        use rand::Rng;
        rand::thread_rng().gen::<Nonce>()
    }
}

impl Bicrypter for Producer {}

impl Encrypter for Producer {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
        Ok(self
            .from_nonce(&Self::make_nonce())
            .map_err(|e| CryptError::Internal(Box::new(e)))?
            .encrypt(data)?)
    }
}

impl Decrypter for Producer {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptError> {
        Ok(self
            .from_nonce(&Self::make_nonce())
            .map_err(|e| CryptError::Internal(Box::new(e)))?
            .decrypt(data)?)
    }
}
