//! Transaction inputs and outputs.

use crate::crypto;
use crate::error::Result;
use crate::wallet::decode_address;
use serde::{Deserialize, Serialize};

/// Marker stored in `prev_index` by coinbase inputs.
pub const COINBASE_INDEX: i64 = -1;

/// Reference to a previous output together with the proof of ownership.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TxIn {
    pub prev_txid: String,
    pub prev_index: i64,
    pub signature: Vec<u8>,
    /// Spender's public key. For a coinbase input it carries a free-form memo.
    pub pub_key: Vec<u8>,
}

impl TxIn {
    /// Hash of the key that signed this input.
    pub fn signer_hash(&self) -> Vec<u8> {
        crypto::hash160(&self.pub_key)
    }
}

/// An amount locked to a public key hash.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TxOut {
    pub value: u64,
    pub pubkey_hash: Vec<u8>,
}

impl TxOut {
    pub fn locked_to(value: u64, address: &str) -> Result<TxOut> {
        Ok(TxOut {
            value,
            pubkey_hash: decode_address(address)?,
        })
    }

    pub fn is_owned_by(&self, pubkey_hash: &[u8]) -> bool {
        self.pubkey_hash == pubkey_hash
    }
}
