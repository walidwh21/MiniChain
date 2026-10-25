//! Unspent transaction output index.
//!
//! Scanning the whole chain for every balance query is slow, so spendable
//! outputs are kept in their own tree: `txid -> slots`. A slot is `None`
//! once spent, which keeps output indexes stable.

use super::block::Block;
use super::ledger::Ledger;
use super::transaction::Transaction;
use super::tx::TxOut;
use crate::error::{err, Result};
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct Slots {
    slots: Vec<Option<TxOut>>,
}

#[derive(Clone)]
pub struct UtxoSet {
    tree: sled::Tree,
    ledger: Ledger,
}

impl UtxoSet {
    pub fn new(store: &Store, ledger: Ledger) -> Result<UtxoSet> {
        Ok(UtxoSet {
            tree: store.utxo()?,
            ledger,
        })
    }

    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Recomputes the index from scratch by replaying the chain oldest first.
    /// Returns the number of transactions that still have unspent outputs.
    pub fn rebuild(&self) -> Result<usize> {
        self.tree.clear()?;
        let mut blocks: Vec<Block> = self.ledger.walk().collect();
        blocks.reverse();
        for block in &blocks {
            self.apply_block(block)?;
        }
        Ok(self.tree.len())
    }

    /// Marks the outputs consumed by `block` as spent and registers the
    /// outputs it creates.
    pub fn apply_block(&self, block: &Block) -> Result<()> {
        for tx in block.transactions() {
            if !tx.is_coinbase() {
                for input in &tx.inputs {
                    let key = input.prev_txid.as_bytes();
                    let raw = match self.tree.get(key)? {
                        Some(raw) => raw,
                        None => {
                            return err(format!(
                                "input refers to unknown or fully spent transaction {}",
                                input.prev_txid
                            ))
                        }
                    };
                    let mut entry: Slots = bincode::deserialize(&raw)?;
                    if let Ok(index) = usize::try_from(input.prev_index) {
                        if index < entry.slots.len() {
                            entry.slots[index] = None;
                        }
                    }
                    if entry.slots.iter().all(|slot| slot.is_none()) {
                        self.tree.remove(key)?;
                    } else {
                        self.tree.insert(key, bincode::serialize(&entry)?)?;
                    }
                }
            }
            let fresh = Slots {
                slots: tx.outputs.iter().cloned().map(Some).collect(),
            };
            self.tree
                .insert(tx.id.as_bytes(), bincode::serialize(&fresh)?)?;
        }
        self.tree.flush()?;
        Ok(())
    }

    /// True when every input of `tx` still points at an unspent output.
    pub fn can_spend(&self, tx: &Transaction) -> Result<bool> {
        if tx.is_coinbase() {
            return Ok(true);
        }
        for input in &tx.inputs {
            let raw = match self.tree.get(input.prev_txid.as_bytes())? {
                Some(raw) => raw,
                None => return Ok(false),
            };
            let entry: Slots = bincode::deserialize(&raw)?;
            let live = usize::try_from(input.prev_index)
                .ok()
                .and_then(|i| entry.slots.get(i))
                .map(|slot| slot.is_some())
                .unwrap_or(false);
            if !live {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Gathers outputs owned by `owner` until `target` is covered (or the
    /// set runs out). Returns the total gathered and `txid -> output indexes`.
    pub fn collect_spendable(
        &self,
        owner: &[u8],
        target: u64,
    ) -> Result<(u64, HashMap<String, Vec<usize>>)> {
        let mut total = 0u64;
        let mut picks: HashMap<String, Vec<usize>> = HashMap::new();
        'scan: for item in self.tree.iter() {
            let (key, raw) = item?;
            let txid = String::from_utf8(key.to_vec())?;
            let entry: Slots = bincode::deserialize(&raw)?;
            for (index, slot) in entry.slots.iter().enumerate() {
                if let Some(output) = slot {
                    if output.is_owned_by(owner) {
                        total += output.value;
                        picks.entry(txid.clone()).or_default().push(index);
                        if total >= target {
                            break 'scan;
                        }
                    }
                }
            }
        }
        Ok((total, picks))
    }

    pub fn unspent_for(&self, owner: &[u8]) -> Result<Vec<TxOut>> {
        let mut found = Vec::new();
        for item in self.tree.iter() {
            let (_, raw) = item?;
            let entry: Slots = bincode::deserialize(&raw)?;
            for output in entry.slots.into_iter().flatten() {
                if output.is_owned_by(owner) {
                    found.push(output);
                }
            }
        }
        Ok(found)
    }

    pub fn balance(&self, owner: &[u8]) -> Result<u64> {
        Ok(self.unspent_for(owner)?.iter().map(|o| o.value).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Keypair;

    struct Fixture {
        ledger: Ledger,
        utxo: UtxoSet,
        alice: Keypair,
        bob: Keypair,
    }

    fn fixture() -> Fixture {
        let store = Store::temporary().unwrap();
        let alice = Keypair::generate().unwrap();
        let bob = Keypair::generate().unwrap();
        let ledger = Ledger::init(&store, &alice.address().unwrap()).unwrap();
        let utxo = UtxoSet::new(&store, ledger.clone()).unwrap();
        utxo.rebuild().unwrap();
        Fixture {
            ledger,
            utxo,
            alice,
            bob,
        }
    }

    #[test]
    fn genesis_reward_is_spendable() {
        let f = fixture();
        assert_eq!(f.utxo.balance(&f.alice.pubkey_hash()).unwrap(), 100);
        assert_eq!(f.utxo.balance(&f.bob.pubkey_hash()).unwrap(), 0);
    }

    #[test]
    fn payment_moves_funds_and_survives_rebuild() {
        let f = fixture();
        let bob_address = f.bob.address().unwrap();
        let tx = Transaction::new_payment(&f.alice, &bob_address, 30, &f.utxo).unwrap();
        assert!(f.ledger.verify_tx(&tx).unwrap());
        assert!(f.utxo.can_spend(&tx).unwrap());

        let reward = Transaction::coinbase(&f.alice.address().unwrap(), "test").unwrap();
        let block = f.ledger.mine_block(vec![reward, tx.clone()]).unwrap();
        f.utxo.apply_block(&block).unwrap();

        assert_eq!(f.utxo.balance(&f.bob.pubkey_hash()).unwrap(), 30);
        assert_eq!(f.utxo.balance(&f.alice.pubkey_hash()).unwrap(), 170);
        assert!(!f.utxo.can_spend(&tx).unwrap());

        f.utxo.rebuild().unwrap();
        assert_eq!(f.utxo.balance(&f.bob.pubkey_hash()).unwrap(), 30);
        assert_eq!(f.utxo.balance(&f.alice.pubkey_hash()).unwrap(), 170);
        assert_eq!(f.ledger.best_height().unwrap(), 1);
    }

    #[test]
    fn overspending_is_refused() {
        let f = fixture();
        let bob_address = f.bob.address().unwrap();
        assert!(Transaction::new_payment(&f.alice, &bob_address, 101, &f.utxo).is_err());
        assert!(Transaction::new_payment(&f.alice, &bob_address, 0, &f.utxo).is_err());
    }

    #[test]
    fn tampered_transaction_fails_verification() {
        let f = fixture();
        let bob_address = f.bob.address().unwrap();
        let mut tx = Transaction::new_payment(&f.alice, &bob_address, 30, &f.utxo).unwrap();
        tx.outputs[0].value = 60;
        assert!(!f.ledger.verify_tx(&tx).unwrap());
        tx.id = tx.compute_id().unwrap();
        assert!(!f.ledger.verify_tx(&tx).unwrap());
    }

    #[test]
    fn thief_cannot_spend_someone_elses_output() {
        let f = fixture();
        let alice_output = f.utxo.unspent_for(&f.alice.pubkey_hash()).unwrap();
        assert_eq!(alice_output.len(), 1);

        // Bob forges a spend of Alice's genesis output using his own key.
        let genesis_txid = f.ledger.walk().last().unwrap().transactions()[0].id.clone();
        let mut forged = Transaction {
            id: String::new(),
            inputs: vec![crate::chain::tx::TxIn {
                prev_txid: genesis_txid,
                prev_index: 0,
                signature: Vec::new(),
                pub_key: f.bob.public.clone(),
            }],
            outputs: vec![TxOut {
                value: 100,
                pubkey_hash: f.bob.pubkey_hash(),
            }],
        };
        f.ledger.sign_tx(&mut forged, &f.bob.secret).unwrap();
        forged.id = forged.compute_id().unwrap();
        assert!(!f.ledger.verify_tx(&forged).unwrap());
    }
}
