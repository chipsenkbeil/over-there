use super::Error as AeadError;
use crate::{nonce, AssociatedData, Bicrypter as BicrypterTrait, CryptError, Decrypter, Encrypter};
use aead::{generic_array::GenericArray, Aead};
use lru::LruCache;
use std::cell::RefCell;

pub struct Bicrypter<T: Aead> {
    aead: T,
    nonce_size: nonce::NonceSize,
    cache: Option<RefCell<LruCache<Vec<u8>, ()>>>,
}

impl<T: Aead> Bicrypter<T> {
    pub fn new(aead: T, nonce_size: nonce::NonceSize, nonce_cache_size: usize) -> Self {
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

    pub fn with_no_nonce_cache(aead: T, nonce_size: nonce::NonceSize) -> Self {
        Self::new(aead, nonce_size, 0)
    }

    fn register_nonce(&self, nonce: &[u8]) -> Result<Vec<u8>, CryptError> {
        if nonce.len() != nonce::nonce_size_to_byte_length(self.nonce_size) {
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

impl<T: Aead> BicrypterTrait for Bicrypter<T> {}

impl<T: Aead> Encrypter for Bicrypter<T> {
    fn encrypt(
        &self,
        buffer: &[u8],
        associated_data: AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        let nonce = associated_data
            .to_nonce()
            .map(|n| self.register_nonce(&n))
            .transpose()?
            .map(|n| GenericArray::clone_from_slice(&n))
            .ok_or(CryptError::MissingNonce)?;
        self.aead
            .encrypt(&nonce, buffer)
            .map_err(|e| CryptError::EncryptFailed(Box::new(AeadError::Aed(e))))
    }
}

impl<T: Aead> Decrypter for Bicrypter<T> {
    fn decrypt(
        &self,
        buffer: &[u8],
        associated_data: AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        let nonce = associated_data
            .to_nonce()
            .map(|n| self.register_nonce(&n))
            .transpose()?
            .map(|n| GenericArray::clone_from_slice(&n))
            .ok_or(CryptError::MissingNonce)?;
        self.aead
            .decrypt(&nonce, buffer)
            .map_err(|e| CryptError::EncryptFailed(Box::new(AeadError::Aed(e))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aead::impls;
}
