use super::{AssociatedData, CryptError, Decrypter, Encrypter};

pub struct Bicrypter;

impl Bicrypter {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::Bicrypter for Bicrypter {}

impl Encrypter for Bicrypter {
    /// Does nothing but return existing data - NoOp
    fn encrypt(&self, buffer: &[u8], _: AssociatedData) -> Result<Vec<u8>, CryptError> {
        Ok(Vec::from(buffer))
    }
}

impl Decrypter for Bicrypter {
    /// Does nothing but return existing data - NoOp
    fn decrypt(&self, buffer: &[u8], _: AssociatedData) -> Result<Vec<u8>, CryptError> {
        Ok(Vec::from(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_should_return_a_copy_of_input_data() {
        let bicrypter = Bicrypter::new();
        let data = vec![1, 2, 3];

        let encrypted_data = bicrypter
            .encrypt(&data, AssociatedData::None)
            .expect("Encrypt failed unexpectedly");
        assert_eq!(data, encrypted_data);
    }

    #[test]
    fn decrypt_should_return_a_copy_of_input_data() {
        let bicrypter = Bicrypter::new();
        let data = vec![1, 2, 3];

        let decrypted_data = bicrypter
            .decrypt(&data, AssociatedData::None)
            .expect("Decrypt failed unexpectedly");
        assert_eq!(data, decrypted_data);
    }
}
