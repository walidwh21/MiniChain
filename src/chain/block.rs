//! Blocks and proof-of-work.

use super::merkle;
use super::transaction::Transaction;
use crate::config::{POW_DIFFICULTY, PROTOCOL_VERSION};
use crate::crypto;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    timestamp: u128,
    hash: String,
    prev_hash: String,
    transactions: Vec<Transaction>,
    nonce: u64,
    height: u64,
    version: u32,
    difficulty: u32,
}

impl Block {
    /// Assembles a block and searches for a nonce that satisfies the target.
    pub fn mine(transactions: Vec<Transaction>, prev_hash: String, height: u64) -> Result<Block> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let mut block = Block {
            timestamp,
            hash: String::new(),
            prev_hash,
            transactions,
            nonce: 0,
            height,
            version: PROTOCOL_VERSION,
            difficulty: POW_DIFFICULTY,
        };
        block.seal()?;
        Ok(block)
    }

    pub fn genesis(coinbase: Transaction) -> Result<Block> {
        Block::mine(vec![coinbase], String::new(), 0)
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn prev_hash(&self) -> &str {
        &self.prev_hash
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub fn timestamp(&self) -> u128 {
        self.timestamp
    }

    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    pub fn difficulty(&self) -> u32 {
        self.difficulty
    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    /// Increments the nonce until the header hash meets the difficulty.
    /// The merkle root is computed once because it does not depend on the nonce.
    fn seal(&mut self) -> Result<()> {
        let root = self.merkle_root();
        let prefix = self.target_prefix();
        loop {
            let digest = self.digest_with(&root)?;
            if digest.starts_with(&prefix) {
                self.hash = digest;
                return Ok(());
            }
            self.nonce += 1;
        }
    }

    fn merkle_root(&self) -> Vec<u8> {
        let leaves: Vec<Vec<u8>> = self
            .transactions
            .iter()
            .map(|tx| tx.id.as_bytes().to_vec())
            .collect();
        merkle::root_of(&leaves)
    }

    fn target_prefix(&self) -> String {
        "0".repeat(self.difficulty as usize)
    }

    fn digest_with(&self, merkle_root: &[u8]) -> Result<String> {
        let header = (
            &self.prev_hash,
            merkle_root,
            self.timestamp,
            self.difficulty,
            self.nonce,
            self.version,
        );
        Ok(crypto::sha256_hex(&bincode::serialize(&header)?))
    }

    /// Structural validity: claimed hash matches the contents, meets the
    /// network difficulty, and every transaction id is genuine.
    pub fn is_valid(&self) -> Result<bool> {
        if self.difficulty < POW_DIFFICULTY {
            return Ok(false);
        }
        for tx in &self.transactions {
            if tx.compute_id()? != tx.id {
                return Ok(false);
            }
        }
        let digest = self.digest_with(&self.merkle_root())?;
        Ok(digest == self.hash && self.hash.starts_with(&self.target_prefix()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Keypair;

    #[test]
    fn mined_block_is_valid_and_tamper_evident() {
        let miner = Keypair::generate().unwrap();
        let coinbase = Transaction::coinbase(&miner.address().unwrap(), "test").unwrap();
        let block = Block::genesis(coinbase).unwrap();
        assert!(block.hash().starts_with("0000"));
        assert!(block.is_valid().unwrap());

        let mut forged = block.clone();
        forged.nonce += 1;
        assert!(!forged.is_valid().unwrap());
    }
}
