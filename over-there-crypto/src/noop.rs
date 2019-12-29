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
