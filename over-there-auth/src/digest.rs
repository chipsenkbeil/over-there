use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use std::convert::TryFrom;

pub type Digest256Bits = [u8; 32];
pub type Digest512Bits = [u8; 64];

big_array! { BigArray; }

#[derive(Serialize, Deserialize, Clone)]
pub enum Digest {
    Digest256Bits(Digest256Bits),

    #[serde(with = "BigArray")]
    Digest512Bits(Digest512Bits),
}

impl Digest {
    /// Retrieves reference to underlying digest
    pub fn digest(&self) -> &[u8] {
        match self {
            Self::Digest256Bits(d) => d,
            Self::Digest512Bits(d) => d,
        }
    }

    /// Verifies the given content with the specified key
    /// given the digest signature (self)
    pub fn verify(&self, key: &[u8], content: &[u8]) -> bool {
        match self {
            Self::Digest256Bits(d) => super::verify_sha256(key, content, d),
            Self::Digest512Bits(d) => super::verify_sha512(key, content, d),
        }
    }
}

impl Default for Digest {
    /// Creates an empty, 256-bit signature
    fn default() -> Self {
        let empty_sig = [0; 32];
        Self::Digest256Bits(empty_sig)
    }
}

impl std::fmt::Debug for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Digest256Bits(d) => write!(f, "Digest256Bits({:?})", d),
            Self::Digest512Bits(d) => {
                let d_str = d
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "Digest512Bits([{}])", d_str)
            }
        }
    }
}

impl From<Digest256Bits> for Digest {
    fn from(digest: Digest256Bits) -> Self {
        Self::Digest256Bits(digest)
    }
}

impl From<Digest512Bits> for Digest {
    fn from(digest: Digest512Bits) -> Self {
        Self::Digest512Bits(digest)
    }
}

impl TryFrom<&[u8]> for Digest {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() == 32 {
            let mut buf: Digest256Bits = [0; 32];
            buf.copy_from_slice(value);
            Ok(From::from(buf))
        } else if value.len() == 64 {
            let mut buf: Digest512Bits = [0; 64];
            buf.copy_from_slice(value);
            Ok(From::from(buf))
        } else {
            Err("Invalid slice length")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as sign;

    #[test]
    fn verify_should_correctly_use_underlying_sha_function() {
        let key = b"some key";
        let msg = b"some message";

        let digest_arr = sign::sign_sha256(key, msg);
        let digest = Digest::from(digest_arr);
        assert!(!digest.verify(b"bad key", msg), "Bad key succeeded");
        assert!(!digest.verify(key, b"bad msg"), "Bad message succeeded");
        assert!(digest.verify(key, msg), "Failed to verify legit message");

        let digest_arr = sign::sign_sha512(key, msg);
        let digest = Digest::from(digest_arr);
        assert!(!digest.verify(b"bad key", msg), "Bad key succeeded");
        assert!(!digest.verify(key, b"bad msg"), "Bad message succeeded");
        assert!(digest.verify(key, msg), "Failed to verify legit message");
    }

    #[test]
    fn default_should_return_empty_256bit_digest() {
        let digest = Digest::default();
        match digest {
            Digest::Digest256Bits(d) => {
                assert_eq!(d, [0; 32], "Created digest was not empty")
            }
            x => panic!("Unexpected digest produced: {:?}", x),
        }
    }

    #[test]
    fn debug_should_properly_yield_digest_strings() {
        let digest: &[u8] = &(0..32).collect::<Vec<u8>>();
        let digest = Digest::try_from(digest).unwrap();
        assert_eq!(
            format!("{:?}", digest),
            "Digest256Bits([\
             0, 1, 2, 3, 4, 5, 6, 7, 8, 9, \
             10, 11, 12, 13, 14, 15, 16, 17, 18, 19, \
             20, 21, 22, 23, 24, 25, 26, 27, 28, 29, \
             30, 31])"
        );

        let digest: &[u8] = &(0..64).collect::<Vec<u8>>();
        let digest = Digest::try_from(digest).unwrap();
        assert_eq!(
            format!("{:?}", digest),
            "Digest512Bits([\
             0, 1, 2, 3, 4, 5, 6, 7, 8, 9, \
             10, 11, 12, 13, 14, 15, 16, 17, 18, 19, \
             20, 21, 22, 23, 24, 25, 26, 27, 28, 29, \
             30, 31, 32, 33, 34, 35, 36, 37, 38, 39, \
             40, 41, 42, 43, 44, 45, 46, 47, 48, 49, \
             50, 51, 52, 53, 54, 55, 56, 57, 58, 59, \
             60, 61, 62, 63])"
        );
    }

    #[test]
    fn from_32_byte_array_should_yield_256bit_digest() {
        let arr: Digest256Bits = [9; 32];
        let digest = Digest::from(arr);
        let digest = digest.digest();
        assert_eq!(digest, arr, "Created digest is different");
    }

    #[test]
    fn from_64_byte_array_should_yield_512bit_digest() {
        let arr: Digest512Bits = [9; 64];
        let digest = Digest::from(arr);
        let digest = digest.digest();

        // NOTE: Ugly check as Rust currently does not generate array impls
        //       above size 32
        for (i, a) in arr.iter().enumerate() {
            assert_eq!(a, &digest[i], "Created digest is different");
        }
    }
}
