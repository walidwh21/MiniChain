//! Toy peer-to-peer layer: a TCP listener that exchanges `Message`s.

mod handlers;
pub mod message;
mod peer;

pub use peer::Peer;

use crate::chain::Transaction;
use crate::error::Result;
use message::{encode, Message, TxMsg};
use std::io::Write;
use std::net::TcpStream;

/// One-shot helper for clients: hand a signed transaction to a running node.
pub fn submit_tx(node: &str, tx: &Transaction) -> Result<()> {
    let bytes = encode(&Message::Tx(TxMsg {
        from: String::new(),
        tx: tx.clone(),
    }))?;
    let mut stream = TcpStream::connect(node)?;
    stream.write_all(&bytes)?;
    Ok(())
}
