//! Key pairs and address encoding.

mod keystore;

pub use keystore::Keystore;

use crate::crypto;
use crate::error::{Error, Result};
use bitcoincash_addr::{Address, HashType, Scheme};
use serde::{Deserialize, Serialize};

/// An ed25519 key pair. The address is derived from the public key hash.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Keypair {
    pub secret: Vec<u8>,
    pub public: Vec<u8>,
}

impl Keypair {
    pub fn generate() -> Result<Keypair> {
        let secret = crypto::new_secret();
        let public = crypto::public_key(&secret)?;
        Ok(Keypair {
            secret: secret.to_vec(),
            public,
        })
    }

    /// 20 byte hash that outputs are locked to.
    pub fn pubkey_hash(&self) -> Vec<u8> {
        crypto::hash160(&self.public)
    }

    pub fn address(&self) -> Result<String> {
        encode_address(&self.pubkey_hash())
    }
}

/// Base58 address for a public key hash.
pub fn encode_address(pubkey_hash: &[u8]) -> Result<String> {
    let address = Address {
        body: pubkey_hash.to_vec(),
        scheme: Scheme::Base58,
        hash_type: HashType::Script,
        ..Default::default()
    };
    address
        .encode()
        .map_err(|e| Error::from(format!("cannot encode address: {:?}", e)))
}

/// Public key hash behind a Base58 address.
pub fn decode_address(address: &str) -> Result<Vec<u8>> {
    Address::decode(address)
        .map(|decoded| decoded.body)
        .map_err(|e| Error::from(format!("invalid address '{}': {:?}", address, e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_round_trip() {
        let pair = Keypair::generate().unwrap();
        let address = pair.address().unwrap();
        assert_eq!(decode_address(&address).unwrap(), pair.pubkey_hash());
        assert_eq!(pair.pubkey_hash().len(), 20);
    }

    #[test]
    fn bad_address_is_an_error() {
        assert!(decode_address("not-an-address").is_err());
    }
}
