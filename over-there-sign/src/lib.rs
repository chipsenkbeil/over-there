mod auth;
pub use auth::{Authenticator, NoopAuthenticator, Sha256Authenticator, Sha512Authenticator};

mod digest;
pub use digest::{Digest, Digest256Bits, Digest512Bits};

use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512};

pub fn sign_sha256(key: &[u8], content: &[u8]) -> Digest256Bits {
    // HMAC can take a key of any size, so we can safely unwrap here
    let mut mac = Hmac::<Sha256>::new_varkey(key).unwrap();
    mac.input(content);

    let result = mac.result();
    let mut buffer = [0; 32];
    buffer.clone_from_slice(result.code().as_slice());
    buffer
}

pub fn verify_sha256(key: &[u8], content: &[u8], code: &Digest256Bits) -> bool {
    // HMAC can take a key of any size, so we can safely unwrap here
    let mut mac = Hmac::<Sha256>::new_varkey(key).unwrap();
    mac.input(content);

    mac.verify(code).is_ok()
}

pub fn sign_sha512(key: &[u8], content: &[u8]) -> Digest512Bits {
    // HMAC can take a key of any size, so we can safely unwrap here
    let mut mac = Hmac::<Sha512>::new_varkey(key).unwrap();
    mac.input(content);

    let result = mac.result();
    let mut buffer = [0; 64];
    buffer.clone_from_slice(result.code().as_slice());
    buffer
}

pub fn verify_sha512(key: &[u8], content: &[u8], code: &Digest512Bits) -> bool {
    // HMAC can take a key of any size, so we can safely unwrap here
    let mut mac = Hmac::<Sha512>::new_varkey(key).unwrap();
    mac.input(content);

    mac.verify(code).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_sign_and_verify_both_work() {
        let key = b"my test key";
        let content = b"some content to sign and verify";

        let code = sign_sha256(key, content);

        assert!(
            verify_sha256(key, content, &code),
            "Failed to verify signed content!",
        );
    }

    #[test]
    fn sha512_sign_and_verify_both_work() {
        let key = b"my test key";
        let content = b"some content to sign and verify";

        let code = sign_sha512(key, content);

        assert!(
            verify_sha512(key, content, &code),
            "Failed to verify signed content!",
        );
    }
}
