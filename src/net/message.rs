//! Wire messages. A frame is simply the bincode encoding of a [`Message`];
//! the sender closes the connection to mark the end of the frame.

use crate::chain::{Block, Transaction};
use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    /// Handshake carrying protocol version and chain height.
    Hello(Hello),
    /// List of peer addresses the sender knows about.
    Peers(Vec<String>),
    /// Announcement of block hashes or transaction ids.
    Inventory(Inventory),
    /// Request for one block or transaction by id.
    GetItem(GetItem),
    /// Ask a peer for its full list of block hashes.
    GetBlocks(GetBlocks),
    /// A full block.
    Block(BlockMsg),
    /// A full transaction.
    Tx(TxMsg),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Block,
    Tx,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hello {
    pub from: String,
    pub version: u32,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Inventory {
    pub from: String,
    pub kind: ItemKind,
    pub ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetItem {
    pub from: String,
    pub kind: ItemKind,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetBlocks {
    pub from: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockMsg {
    pub from: String,
    pub block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TxMsg {
    pub from: String,
    pub tx: Transaction,
}

pub fn encode(message: &Message) -> Result<Vec<u8>> {
    Ok(bincode::serialize(message)?)
}

pub fn decode(bytes: &[u8]) -> Result<Message> {
    Ok(bincode::deserialize(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let original = Message::Inventory(Inventory {
            from: "127.0.0.1:3000".to_string(),
            kind: ItemKind::Block,
            ids: vec!["abc".to_string(), "def".to_string()],
        });
        let bytes = encode(&original).unwrap();
        match decode(&bytes).unwrap() {
            Message::Inventory(inv) => {
                assert_eq!(inv.kind, ItemKind::Block);
                assert_eq!(inv.ids.len(), 2);
            }
            other => panic!("unexpected message: {:?}", other),
        }
    }

    #[test]
    fn junk_does_not_decode() {
        assert!(decode(&[0xff, 0xff, 0xff]).is_err());
    }
}
