use crate::{AssociatedData, Bicrypter, CryptError, Decrypter, Encrypter};

#[derive(Clone, Copy)]
pub struct NoopBicrypter;

impl Bicrypter for NoopBicrypter {}

impl Encrypter for NoopBicrypter {
    /// Does nothing but return existing data - NoOp
    fn encrypt(
        &self,
        buffer: &[u8],
        _: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        Ok(Vec::from(buffer))
    }

    /// Returns no associated data
    fn new_encrypt_associated_data(&self) -> AssociatedData {
        AssociatedData::None
    }
}

impl Decrypter for NoopBicrypter {
    /// Does nothing but return existing data - NoOp
    fn decrypt(
        &self,
        buffer: &[u8],
        _: &AssociatedData,
    ) -> Result<Vec<u8>, CryptError> {
        Ok(Vec::from(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_should_return_a_copy_of_input_data() {
        let bicrypter = NoopBicrypter;
        let data = vec![1, 2, 3];

        let encrypted_data = bicrypter
            .encrypt(&data, &AssociatedData::None)
            .expect("Encrypt failed unexpectedly");
        assert_eq!(data, encrypted_data);
    }

    #[test]
    fn decrypt_should_return_a_copy_of_input_data() {
        let bicrypter = NoopBicrypter;
        let data = vec![1, 2, 3];

        let decrypted_data = bicrypter
            .decrypt(&data, &AssociatedData::None)
            .expect("Decrypt failed unexpectedly");
        assert_eq!(data, decrypted_data);
    }
}
