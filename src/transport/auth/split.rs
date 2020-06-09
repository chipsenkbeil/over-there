use super::{Authenticator, Digest, Signer, Verifier};
use std::sync::Arc;

/// Splits authenticator into signer and verifier halves that point to
/// the same underlying authenticator via arc
pub fn split<A>(authenticator: A) -> (SignerHalf<A>, VerifierHalf<A>)
where
    A: Authenticator,
{
    let arc_self = Arc::new(authenticator);
    let arc_self_2 = Arc::clone(&arc_self);

    let s_half = SignerHalf { signer: arc_self };
    let v_half = VerifierHalf {
        verifier: arc_self_2,
    };
    (s_half, v_half)
}

/// Splits authenticator into signer and verifier halves by cloning the
/// authenticator
pub fn clone_split<A>(original: A) -> (A, A)
where
    A: Authenticator + Clone,
{
    let clone = original.clone();
    (original, clone)
}

pub struct SignerHalf<S>
where
    S: Signer,
{
    signer: Arc<S>,
}

impl<S> Signer for SignerHalf<S>
where
    S: Signer,
{
    fn sign(&self, message: &[u8]) -> Digest {
        self.signer.sign(message)
    }
}

pub struct VerifierHalf<V>
where
    V: Verifier,
{
    verifier: Arc<V>,
}

impl<V> Verifier for VerifierHalf<V>
where
    V: Verifier,
{
    fn verify(&self, message: &[u8], signature: &Digest) -> bool {
        self.verifier.verify(message, signature)
    }
}
