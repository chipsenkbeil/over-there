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
    AesGcm128,
    AesGcm256,
    AesGcmSiv128,
    AesGcmSiv256,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumString, EnumVariantNames)]
pub enum Transport {
    Tcp,
    Udp,
}
