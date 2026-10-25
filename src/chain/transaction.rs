//! Transactions: construction, signing and verification.

use super::tx::{TxIn, TxOut, COINBASE_INDEX};
use super::utxo::UtxoSet;
use crate::config::BLOCK_REWARD;
use crate::crypto;
use crate::error::{err, Result};
use crate::wallet::Keypair;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub id: String,
    pub inputs: Vec<TxIn>,
    pub outputs: Vec<TxOut>,
}

impl Transaction {
    /// Creates new coins for `to`. A random salt keeps ids unique even when
    /// the same address is rewarded twice with the same memo.
    pub fn coinbase(to: &str, memo: &str) -> Result<Transaction> {
        let salt = OsRng.next_u64();
        let note = if memo.is_empty() {
            format!("reward to {} [{:016x}]", to, salt)
        } else {
            format!("{} [{:016x}]", memo, salt)
        };
        let mut tx = Transaction {
            id: String::new(),
            inputs: vec![TxIn {
                prev_txid: String::new(),
                prev_index: COINBASE_INDEX,
                signature: Vec::new(),
                pub_key: note.into_bytes(),
            }],
            outputs: vec![TxOut::locked_to(BLOCK_REWARD, to)?],
        };
        tx.id = tx.compute_id()?;
        Ok(tx)
    }

    /// Builds and signs a payment from `sender` to the address `to`.
    ///
    /// Enough unspent outputs are gathered to cover `amount`; any surplus is
    /// returned to the sender as a change output.
    pub fn new_payment(
        sender: &Keypair,
        to: &str,
        amount: u64,
        utxo: &UtxoSet,
    ) -> Result<Transaction> {
        if amount == 0 {
            return err("amount must be greater than zero");
        }
        let sender_hash = sender.pubkey_hash();
        let (available, picks) = utxo.collect_spendable(&sender_hash, amount)?;
        if available < amount {
            return err(format!(
                "insufficient funds: {} available, {} requested",
                available, amount
            ));
        }

        let mut inputs = Vec::new();
        for (txid, indexes) in picks {
            for index in indexes {
                inputs.push(TxIn {
                    prev_txid: txid.clone(),
                    prev_index: index as i64,
                    signature: Vec::new(),
                    pub_key: sender.public.clone(),
                });
            }
        }

        let mut outputs = vec![TxOut::locked_to(amount, to)?];
        if available > amount {
            outputs.push(TxOut {
                value: available - amount,
                pubkey_hash: sender_hash,
            });
        }

        let mut tx = Transaction {
            id: String::new(),
            inputs,
            outputs,
        };
        utxo.ledger().sign_tx(&mut tx, &sender.secret)?;
        tx.id = tx.compute_id()?;
        Ok(tx)
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1
            && self.inputs[0].prev_txid.is_empty()
            && self.inputs[0].prev_index == COINBASE_INDEX
    }

    /// Id = SHA-256 over the whole transaction with the id field blanked.
    pub fn compute_id(&self) -> Result<String> {
        let mut bare = self.clone();
        bare.id.clear();
        Ok(crypto::sha256_hex(&bincode::serialize(&bare)?))
    }

    /// The bytes that input `input_index` signs: the transaction with every
    /// signature stripped and only that input carrying the hash of the output
    /// it spends.
    fn signing_digest(&self, input_index: usize, locking_hash: &[u8]) -> Result<Vec<u8>> {
        let mut view = self.clone();
        view.id.clear();
        for (position, input) in view.inputs.iter_mut().enumerate() {
            input.signature.clear();
            input.pub_key = if position == input_index {
                locking_hash.to_vec()
            } else {
                Vec::new()
            };
        }
        Ok(crypto::sha256(&bincode::serialize(&view)?))
    }

    /// Signs every input with `secret`. `prev` maps txid -> the transactions
    /// being spent.
    pub fn sign(&mut self, secret: &[u8], prev: &HashMap<String, Transaction>) -> Result<()> {
        if self.is_coinbase() {
            return Ok(());
        }
        for index in 0..self.inputs.len() {
            let locking_hash = {
                let input = &self.inputs[index];
                let source = match prev.get(&input.prev_txid) {
                    Some(source) => source,
                    None => return err(format!("unknown previous transaction {}", input.prev_txid)),
                };
                match usize::try_from(input.prev_index)
                    .ok()
                    .and_then(|i| source.outputs.get(i))
                {
                    Some(output) => output.pubkey_hash.clone(),
                    None => return err("previous output index out of range"),
                }
            };
            let digest = self.signing_digest(index, &locking_hash)?;
            self.inputs[index].signature = crypto::sign(secret, &digest)?;
        }
        Ok(())
    }

    /// Full validity check: id, ownership, signatures and value balance.
    pub fn verify(&self, prev: &HashMap<String, Transaction>) -> Result<bool> {
        if self.id != self.compute_id()? {
            return Ok(false);
        }
        if self.is_coinbase() {
            let minted: u128 = self.outputs.iter().map(|o| o.value as u128).sum();
            return Ok(minted == BLOCK_REWARD as u128);
        }

        let mut total_in: u128 = 0;
        for (index, input) in self.inputs.iter().enumerate() {
            let source = match prev.get(&input.prev_txid) {
                Some(source) => source,
                None => return Ok(false),
            };
            let spent = match usize::try_from(input.prev_index)
                .ok()
                .and_then(|i| source.outputs.get(i))
            {
                Some(output) => output,
                None => return Ok(false),
            };
            if input.signer_hash() != spent.pubkey_hash {
                return Ok(false);
            }
            let digest = self.signing_digest(index, &spent.pubkey_hash)?;
            if !crypto::verify(&input.pub_key, &digest, &input.signature) {
                return Ok(false);
            }
            total_in += spent.value as u128;
        }
        let total_out: u128 = self.outputs.iter().map(|o| o.value as u128).sum();
        Ok(total_in >= total_out)
    }
}
