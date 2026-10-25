//! minichain - a small educational blockchain.
//!
//! The binary is a thin shell around the library-style modules below:
//! `wallet` (keys), `chain` (blocks, transactions, UTXO set), `net` (peer
//! protocol) and `cli` (the command line front end).

mod chain;
mod cli;
mod config;
mod crypto;
mod error;
mod net;
mod store;
mod wallet;

fn main() {
    env_logger::init();
    if let Err(problem) = cli::run() {
        eprintln!("error: {}", problem);
        std::process::exit(1);
    }
}
