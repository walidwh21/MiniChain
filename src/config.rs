//! Tunable constants and environment lookups.

/// Version number stamped into blocks and handshake messages.
pub const PROTOCOL_VERSION: u32 = 1;

/// Number of leading zero hex digits a block hash must have.
pub const POW_DIFFICULTY: u32 = 4;

/// Coins created by every coinbase transaction.
pub const BLOCK_REWARD: u64 = 100;

/// Well-known node every other node introduces itself to.
pub const SEED_NODE: &str = "127.0.0.1:3000";

/// Upper bound for a single inbound network message.
pub const MAX_MESSAGE_BYTES: u64 = 16 * 1024 * 1024;

/// Directory holding the database. Override with `MINICHAIN_DATA`.
pub fn data_dir() -> String {
    std::env::var("MINICHAIN_DATA").unwrap_or_else(|_| String::from("data"))
}
