use strum_macros::{EnumString, EnumVariantNames};

#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumString, EnumVariantNames)]
pub enum Authentication {
    None,
    Sha256,
    Sha512,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumString, EnumVariantNames)]
pub enum Encryption {
    None,
    Aes128Gcm,
    Aes256Gcm,
    Aes128GcmSiv,
    Aes256GcmSiv,
    Aes128Siv,
    Aes256Siv,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumString, EnumVariantNames)]
pub enum Transport {
    Tcp,
    Udp,
}
