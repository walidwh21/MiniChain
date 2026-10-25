//! Hashing and signature primitives used across the crate.

use crate::error::{Error, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

/// SHA-256 as a lowercase hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

/// SHA-256 as raw bytes.
pub fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}

/// RIPEMD-160(SHA-256(data)), the classic 20 byte "hash160".
pub fn hash160(data: &[u8]) -> Vec<u8> {
    Ripemd160::digest(Sha256::digest(data)).to_vec()
}

/// Fresh 32 byte secret from the operating system RNG.
pub fn new_secret() -> [u8; 32] {
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    secret
}

fn signing_key(secret: &[u8]) -> Result<SigningKey> {
    let bytes: [u8; 32] = secret
        .try_into()
        .map_err(|_| Error::from("secret key must be exactly 32 bytes"))?;
    Ok(SigningKey::from_bytes(&bytes))
}

/// Derives the 32 byte public key belonging to `secret`.
pub fn public_key(secret: &[u8]) -> Result<Vec<u8>> {
    Ok(signing_key(secret)?.verifying_key().to_bytes().to_vec())
}

/// Signs `message`, returning the 64 byte signature.
pub fn sign(secret: &[u8], message: &[u8]) -> Result<Vec<u8>> {
    let signature: Signature = signing_key(secret)?.sign(message);
    Ok(signature.to_bytes().to_vec())
}

/// Checks a signature; any malformed input simply counts as invalid.
pub fn verify(public: &[u8], message: &[u8], signature: &[u8]) -> bool {
    let key_bytes = match <[u8; 32]>::try_from(public) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let key = match VerifyingKey::from_bytes(&key_bytes) {
        Ok(key) => key,
        Err(_) => return false,
    };
    let signature = match Signature::try_from(signature) {
        Ok(signature) => signature,
        Err(_) => return false,
    };
    key.verify(message, &signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash160_is_twenty_bytes() {
        assert_eq!(hash160(b"hello").len(), 20);
    }

    #[test]
    fn sign_then_verify() {
        let secret = new_secret();
        let public = public_key(&secret).unwrap();
        let signature = sign(&secret, b"payload").unwrap();
        assert!(verify(&public, b"payload", &signature));
        assert!(!verify(&public, b"other payload", &signature));
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(!verify(&[1, 2, 3], b"x", &[4, 5, 6]));
    }
}
