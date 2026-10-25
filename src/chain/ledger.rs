//! The block store and chain head.

use super::block::Block;
use super::transaction::Transaction;
use crate::error::{err, Result};
use crate::store::Store;
use log::info;
use std::collections::{HashMap, HashSet};

const HEAD_KEY: &[u8] = b"head";

/// Handle to the persisted chain. Cloning is cheap and all clones see the
/// same data.
#[derive(Clone)]
pub struct Ledger {
    blocks: sled::Tree,
    meta: sled::Tree,
}

impl Ledger {
    /// Opens an existing chain; fails if none has been created yet.
    pub fn load(store: &Store) -> Result<Ledger> {
        let ledger = Ledger {
            blocks: store.blocks()?,
            meta: store.meta()?,
        };
        if ledger.head_hash()?.is_none() {
            return err("no chain found - create one first with `chain init <ADDRESS>`");
        }
        Ok(ledger)
    }

    /// Wipes any existing chain and starts a new one whose genesis reward
    /// goes to `reward_address`.
    pub fn init(store: &Store, reward_address: &str) -> Result<Ledger> {
        let ledger = Ledger {
            blocks: store.blocks()?,
            meta: store.meta()?,
        };
        ledger.blocks.clear()?;
        ledger.meta.clear()?;

        let coinbase = Transaction::coinbase(reward_address, "genesis block")?;
        let genesis = Block::genesis(coinbase)?;
        ledger.store_block(&genesis)?;
        ledger.set_head(genesis.hash())?;
        ledger.flush()?;
        info!("created chain, genesis {}", genesis.hash());
        Ok(ledger)
    }

    fn flush(&self) -> Result<()> {
        self.blocks.flush()?;
        self.meta.flush()?;
        Ok(())
    }

    fn store_block(&self, block: &Block) -> Result<()> {
        self.blocks
            .insert(block.hash().as_bytes(), bincode::serialize(block)?)?;
        Ok(())
    }

    fn set_head(&self, hash: &str) -> Result<()> {
        self.meta.insert(HEAD_KEY, hash.as_bytes())?;
        Ok(())
    }

    pub fn head_hash(&self) -> Result<Option<String>> {
        match self.meta.get(HEAD_KEY)? {
            Some(raw) => Ok(Some(String::from_utf8(raw.to_vec())?)),
            None => Ok(None),
        }
    }

    pub fn get_block(&self, hash: &str) -> Result<Option<Block>> {
        match self.blocks.get(hash.as_bytes())? {
            Some(raw) => Ok(Some(bincode::deserialize(&raw)?)),
            None => Ok(None),
        }
    }

    pub fn has_block(&self, hash: &str) -> Result<bool> {
        Ok(self.blocks.contains_key(hash.as_bytes())?)
    }

    pub fn best_height(&self) -> Result<u64> {
        let head = match self.head_hash()? {
            Some(head) => head,
            None => return err("chain is empty"),
        };
        match self.get_block(&head)? {
            Some(block) => Ok(block.height()),
            None => err("head block is missing from storage"),
        }
    }

    /// Iterates from the newest block back to genesis.
    pub fn walk(&self) -> LedgerWalker {
        LedgerWalker {
            blocks: self.blocks.clone(),
            cursor: self.head_hash().ok().flatten(),
        }
    }

    /// Hashes of all blocks, newest first.
    pub fn block_hashes(&self) -> Vec<String> {
        self.walk().map(|b| b.hash().to_string()).collect()
    }

    /// Linear scan for a transaction by id.
    pub fn find_tx(&self, id: &str) -> Result<Option<Transaction>> {
        for block in self.walk() {
            for tx in block.transactions() {
                if tx.id == id {
                    return Ok(Some(tx.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Looks up the transactions referenced by the inputs of `tx`.
    /// References that cannot be found are simply left out.
    pub fn collect_prev(&self, tx: &Transaction) -> Result<HashMap<String, Transaction>> {
        let mut found = HashMap::new();
        for input in &tx.inputs {
            if let Some(previous) = self.find_tx(&input.prev_txid)? {
                found.insert(previous.id.clone(), previous);
            }
        }
        Ok(found)
    }

    pub fn sign_tx(&self, tx: &mut Transaction, secret: &[u8]) -> Result<()> {
        let prev = self.collect_prev(tx)?;
        tx.sign(secret, &prev)
    }

    pub fn verify_tx(&self, tx: &Transaction) -> Result<bool> {
        if tx.is_coinbase() {
            return tx.verify(&HashMap::new());
        }
        let prev = self.collect_prev(tx)?;
        tx.verify(&prev)
    }

    /// Stores a block received from elsewhere. Returns `false` when it was
    /// already known. The head only moves if the block is taller.
    pub fn append_block(&self, block: Block) -> Result<bool> {
        if self.has_block(block.hash())? {
            return Ok(false);
        }
        if !block.is_valid()? {
            return err(format!("rejected invalid block {}", block.hash()));
        }
        for tx in block.transactions() {
            if !self.verify_tx(tx)? {
                return err(format!(
                    "block {} holds an invalid transaction",
                    block.hash()
                ));
            }
        }
        self.store_block(&block)?;
        if block.height() > self.best_height()? {
            self.set_head(block.hash())?;
        }
        self.flush()?;
        Ok(true)
    }

    /// Verifies `transactions`, mines them into a new block on top of the
    /// current head and persists it.
    pub fn mine_block(&self, transactions: Vec<Transaction>) -> Result<Block> {
        let mut claimed: HashSet<(String, i64)> = HashSet::new();
        for tx in &transactions {
            if !self.verify_tx(tx)? {
                return err(format!("transaction {} failed verification", tx.id));
            }
            if tx.is_coinbase() {
                continue;
            }
            for input in &tx.inputs {
                if !claimed.insert((input.prev_txid.clone(), input.prev_index)) {
                    return err("two transactions in one block spend the same output");
                }
            }
        }

        let head = match self.head_hash()? {
            Some(head) => head,
            None => return err("chain is empty"),
        };
        let height = self.best_height()? + 1;
        info!(
            "mining block #{} with {} transactions",
            height,
            transactions.len()
        );
        let block = Block::mine(transactions, head, height)?;
        self.store_block(&block)?;
        self.set_head(block.hash())?;
        self.flush()?;
        Ok(block)
    }
}

/// Walks the chain backwards, one block per call to `next`.
pub struct LedgerWalker {
    blocks: sled::Tree,
    cursor: Option<String>,
}

impl Iterator for LedgerWalker {
    type Item = Block;

    fn next(&mut self) -> Option<Block> {
        let hash = self.cursor.take()?;
        let raw = self.blocks.get(hash.as_bytes()).ok()??;
        let block: Block = bincode::deserialize(&raw).ok()?;
        if !block.prev_hash().is_empty() {
            self.cursor = Some(block.prev_hash().to_string());
        }
        Some(block)
    }
}
