use crate::{AssociatedData, CryptError, Decrypter, Encrypter};

#[derive(Clone)]
pub struct ClosureEncrypter<F>
where
    F: Fn(&[u8], &AssociatedData) -> Result<Vec<u8>, CryptError> + Clone,
{
    f: F,
}

impl<F> ClosureEncrypter<F>
where
    F: Fn(&[u8], &AssociatedData) -> Result<Vec<u8>, CryptError> + Clone,
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Encrypter for ClosureEncrypter<F>
where
    F: Fn(&[u8], &AssociatedData) -> Result<Vec<u8>, CryptError> + Clone,
{
    /// Does nothing but return existing data - NoOp
    fn encrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        (self.f)(buffer, associated_data)
    }

    /// Returns no associated data
    fn new_encrypt_associated_data(&self) -> AssociatedData {
        AssociatedData::None
    }
}

#[derive(Clone)]
pub struct ClosureDecrypter<F>
where
    F: Fn(&[u8], &AssociatedData) -> Result<Vec<u8>, CryptError> + Clone,
{
    f: F,
}

impl<F> ClosureDecrypter<F>
where
    F: Fn(&[u8], &AssociatedData) -> Result<Vec<u8>, CryptError> + Clone,
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Decrypter for ClosureDecrypter<F>
where
    F: Fn(&[u8], &AssociatedData) -> Result<Vec<u8>, CryptError> + Clone,
{
    fn decrypt(
        &self,
        buffer: &[u8],
        associated_data: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        (self.f)(buffer, associated_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nonce;

    #[test]
    fn closure_encrypter_encrypt_should_return_a_copy_of_input_data() {
        let encrypter = ClosureEncrypter::new(|data, associated_data| {
            let mut v = Vec::new();

            for d in data {
                v.push(*d);
            }

            if let AssociatedData::Nonce(nonce) = associated_data {
                for d in nonce.as_slice() {
                    v.push(*d);
                }
            }

            Ok(v)
        });
        let data = vec![1, 2, 3];
        let associated_data =
            AssociatedData::Nonce(From::from(nonce::new_96bit_nonce()));

        let encrypted_data = encrypter
            .encrypt(&data, &associated_data)
            .expect("Decrypt failed unexpectedly");
        assert_eq!(
            [&vec![1, 2, 3], associated_data.nonce_slice().unwrap()].concat(),
            encrypted_data
        );
    }

    #[test]
    fn closure_decrypter_decrypt_should_return_a_copy_of_input_data() {
        let decrypter = ClosureDecrypter::new(|data, associated_data| {
            let mut v = Vec::new();

            for d in data {
                v.push(*d);
            }

            if let AssociatedData::Nonce(nonce) = associated_data {
                for d in nonce.as_slice() {
                    v.push(*d);
                }
            }

            Ok(v)
        });
        let data = vec![1, 2, 3];
        let associated_data =
            AssociatedData::Nonce(From::from(nonce::new_96bit_nonce()));

        let decrypted_data = decrypter
            .decrypt(&data, &associated_data)
            .expect("Decrypt failed unexpectedly");
        assert_eq!(
            [&vec![1, 2, 3], associated_data.nonce_slice().unwrap()].concat(),
            decrypted_data
        );
    }
}
