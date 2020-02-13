use super::Digest;

pub trait Authenticator: Signer + Verifier {}

pub trait Signer {
    /// Signs some some message, producing a digest
    fn sign(&self, message: &[u8]) -> Digest;
}

pub trait Verifier {
    /// Verifies a signature (digest) for some message
    fn verify(&self, message: &[u8], signature: &Digest) -> bool;
}

pub struct NoopAuthenticator;

impl Authenticator for NoopAuthenticator {}

impl Signer for NoopAuthenticator {
    /// Signs some some message, producing a digest
    fn sign(&self, _message: &[u8]) -> Digest {
        Digest::default()
    }
}

impl Verifier for NoopAuthenticator {
    /// Verifies a signature (digest) for some message
    fn verify(&self, _message: &[u8], _signature: &Digest) -> bool {
        true
    }
}

pub struct ClosureSigner<F>
where
    F: Fn(&[u8]) -> Digest,
{
    f: F,
}

impl<F> ClosureSigner<F>
where
    F: Fn(&[u8]) -> Digest,
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Signer for ClosureSigner<F>
where
    F: Fn(&[u8]) -> Digest,
{
    fn sign(&self, message: &[u8]) -> Digest {
        (self.f)(message)
    }
}

pub struct ClosureVerifier<F>
where
    F: Fn(&[u8], &Digest) -> bool,
{
    f: F,
}

impl<F> ClosureVerifier<F>
where
    F: Fn(&[u8], &Digest) -> bool,
{
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Verifier for ClosureVerifier<F>
where
    F: Fn(&[u8], &Digest) -> bool,
{
    fn verify(&self, message: &[u8], signature: &Digest) -> bool {
        (self.f)(message, signature)
    }
}

pub struct Sha256Authenticator {
    key: Vec<u8>,
}

impl Sha256Authenticator {
    pub fn new(key: &[u8]) -> Self {
        Self { key: key.to_vec() }
    }
}

impl Authenticator for Sha256Authenticator {}

impl Signer for Sha256Authenticator {
    /// Signs some some message, producing a digest
    fn sign(&self, message: &[u8]) -> Digest {
        From::from(super::sign_sha256(&self.key, message))
    }
}

impl Verifier for Sha256Authenticator {
    /// Verifies a signature (digest) for some message
    fn verify(&self, message: &[u8], signature: &Digest) -> bool {
        signature.verify(&self.key, message)
    }
}

pub struct Sha512Authenticator {
    key: Vec<u8>,
}

impl Sha512Authenticator {
    pub fn new(key: &[u8]) -> Self {
        Self { key: key.to_vec() }
    }
}

impl Authenticator for Sha512Authenticator {}

impl Signer for Sha512Authenticator {
    /// Signs some some message, producing a digest
    fn sign(&self, message: &[u8]) -> Digest {
        From::from(super::sign_sha512(&self.key, message))
    }
}

impl Verifier for Sha512Authenticator {
    /// Verifies a signature (digest) for some message
    fn verify(&self, message: &[u8], signature: &Digest) -> bool {
        signature.verify(&self.key, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_signer_should_use_function_to_sign_message() {
        use std::convert::TryFrom;
        let signer = ClosureSigner::new(|msg| Digest::try_from(msg).unwrap());

        let msg = [9; 32];
        let digest = signer.sign(&msg);
        assert_eq!(digest.digest(), &msg);
    }

    #[test]
    fn closure_verifier_should_use_function_to_verify_message() {
        let verifier = ClosureVerifier::new(|msg, _| msg == b"good msg");

        assert!(verifier.verify(b"good msg", &Digest::default()));
        assert!(!verifier.verify(b"bad msg", &Digest::default()));
    }

    #[test]
    fn sha256_auth_key_returns_correct_key() {
        let key = b"my secret key";
        let auth = Sha256Authenticator::new(key);
        assert_eq!(key.to_vec(), auth.key);
    }

    #[test]
    fn sha256_auth_sign_should_produce_256_bit_digest() {
        let key = b"my secret key";
        let auth = Sha256Authenticator::new(key);
        let digest = auth.sign(b"some message");
        match digest {
            Digest::Digest256Bits(_) => (),
            x => panic!("Unexpected digest: {:?}", x),
        }
    }

    #[test]
    fn sha256_auth_verify_should_yield_false_on_bad_message() {
        let key = b"my secret key";
        let auth = Sha256Authenticator::new(key);

        let msg = b"some message";
        let bad_sig = &Digest::from([0; 32]);
        assert!(!auth.verify(msg, bad_sig), "Bad signature succeeded");
    }

    #[test]
    fn sha256_auth_verify_should_yield_true_on_good_message() {
        let key = b"my secret key";
        let auth = Sha256Authenticator::new(key);

        let msg = b"some message";
        let sig = &auth.sign(msg);
        assert!(auth.verify(msg, sig), "Good signature failed");
    }

    #[test]
    fn sha512_auth_key_returns_correct_key() {
        let key = b"my secret key";
        let auth = Sha512Authenticator::new(key);
        assert_eq!(key.to_vec(), auth.key);
    }

    #[test]
    fn sha512_auth_sign_should_produce_512_bit_digest() {
        let key = b"my secret key";
        let auth = Sha512Authenticator::new(key);
        let digest = auth.sign(b"some message");
        match digest {
            Digest::Digest512Bits(_) => (),
            x => panic!("Unexpected digest: {:?}", x),
        }
    }

    #[test]
    fn sha512_auth_verify_should_yield_false_on_bad_message() {
        let key = b"my secret key";
        let auth = Sha512Authenticator::new(key);

        let msg = b"some message";
        let bad_sig = &Digest::from([0; 32]);
        assert!(!auth.verify(msg, bad_sig), "Bad signature succeeded");
    }

    #[test]
    fn sha512_auth_verify_should_yield_true_on_good_message() {
        let key = b"my secret key";
        let auth = Sha512Authenticator::new(key);

        let msg = b"some message";
        let sig = &auth.sign(msg);
        assert!(auth.verify(msg, sig), "Good signature failed");
    }
}
