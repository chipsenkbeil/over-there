use super::{Decrypter, Encrypter, Error};

pub struct Bicrypter;

impl Bicrypter {
    pub fn new() -> Self {
        Self {}
    }
}

impl super::Bicrypter for Bicrypter {}

impl Encrypter for Bicrypter {
    /// Does nothing but return existing data - NoOp
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(Vec::from(data))
    }
}

impl Decrypter for Bicrypter {
    /// Does nothing but return existing data - NoOp
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        Ok(Vec::from(data))
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
            .encrypt(&data)
            .expect("Encrypt failed unexpectedly");
        assert_eq!(data, encrypted_data);
    }

    #[test]
    fn decrypt_should_return_a_copy_of_input_data() {
        let bicrypter = Bicrypter::new();
        let data = vec![1, 2, 3];

        let decrypted_data = bicrypter
            .decrypt(&data)
            .expect("Decrypt failed unexpectedly");
        assert_eq!(data, decrypted_data);
    }
}
