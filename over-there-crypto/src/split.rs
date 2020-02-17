use super::{AssociatedData, Bicrypter, CryptError, Decrypter, Encrypter};
use std::sync::Arc;

/// Splits bicrypter into encrypter and decrypter halves that point to
/// the same underlying bicrypter via arc
pub fn split<B>(bicrypter: B) -> (EncrypterHalf<B>, DecrypterHalf<B>)
where
    B: Bicrypter,
{
    let arc_self = Arc::new(bicrypter);
    let arc_self_2 = Arc::clone(&arc_self);

    let e_half = EncrypterHalf {
        encrypter: arc_self,
    };
    let d_half = DecrypterHalf {
        decrypter: arc_self_2,
    };
    (e_half, d_half)
}

/// Splits bicrypter into encryper and decrypter halves by cloning the bicrypter,
pub fn clone_split<B>(bicrypter: B) -> (B, B)
where
    B: Bicrypter + Clone,
{
    let bicrypter_clone = bicrypter.clone();
    (bicrypter, bicrypter_clone)
}

pub struct EncrypterHalf<E>
where
    E: Encrypter,
{
    encrypter: Arc<E>,
}

impl<E> Encrypter for EncrypterHalf<E>
where
    E: Encrypter,
{
    fn encrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        self.encrypter.encrypt(buffer, associated_data)
    }

    fn new_encrypt_associated_data(&self) -> AssociatedData {
        self.encrypter.new_encrypt_associated_data()
    }
}

pub struct DecrypterHalf<D>
where
    D: Decrypter,
{
    decrypter: Arc<D>,
}

impl<D> Decrypter for DecrypterHalf<D>
where
    D: Decrypter,
{
    fn decrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        self.decrypter.decrypt(buffer, associated_data)
    }
}
