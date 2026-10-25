//! Human readable chain dump.

use crate::chain::{Block, Ledger, Transaction};
use crate::wallet::encode_address;

pub fn print_chain(ledger: &Ledger) {
    for block in ledger.walk() {
        print_block(&block);
        println!();
    }
}

fn print_block(block: &Block) {
    println!("Block #{}", block.height());
    println!("  hash       {}", block.hash());
    if block.prev_hash().is_empty() {
        println!("  prev       (genesis)");
    } else {
        println!("  prev       {}", block.prev_hash());
    }
    println!("  timestamp  {} ms", block.timestamp());
    println!(
        "  nonce      {}   difficulty {}",
        block.nonce(),
        block.difficulty()
    );
    println!("  transactions ({})", block.transactions().len());
    for tx in block.transactions() {
        print_tx(tx);
    }
}

fn print_tx(tx: &Transaction) {
    if tx.is_coinbase() {
        let memo = String::from_utf8_lossy(&tx.inputs[0].pub_key).to_string();
        println!("    [coinbase] {}", abbreviate(&tx.id));
        println!("        memo  {}", memo);
    } else {
        println!("    [transfer] {}", abbreviate(&tx.id));
        for input in &tx.inputs {
            let signer = encode_address(&input.signer_hash())
                .unwrap_or_else(|_| String::from("<invalid>"));
            println!(
                "        in    {}:{}  signed by {}",
                abbreviate(&input.prev_txid),
                input.prev_index,
                signer
            );
        }
    }
    for output in &tx.outputs {
        let owner = encode_address(&output.pubkey_hash)
            .unwrap_or_else(|_| String::from("<invalid>"));
        println!("        out   {:>4}  ->  {}", output.value, owner);
    }
}

/// First and last eight characters of a long hex string.
fn abbreviate(value: &str) -> String {
    if value.len() > 20 && value.is_ascii() {
        format!("{}..{}", &value[..8], &value[value.len() - 8..])
    } else {
        value.to_string()
    }
}
