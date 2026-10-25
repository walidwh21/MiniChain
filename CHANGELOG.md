# Changelog

## 0.2.0

Internal rewrite; the command line surface was regrouped.

- New module layout: `wallet`, `chain`, `net`, `cli`, plus `crypto`, `store`, `config`, `error`.
- A single `sled` database with named trees replaces separate databases per concern.
- Crypto moved to maintained crates (`sha2`, `ripemd`, `ed25519-dalek`); addresses now use hash160.
- Shared `Result` / `Error` types replace the `failure` crate.
- Chain head is tracked under one key; previously two different keys were used.
- UTXO slots keep stable output indexes (spent slots become `None`).
- Transaction verification now also checks ownership of the spent output, the transaction id and value balance.
- Coinbase transactions carry a random salt so identical rewards get distinct ids.
- Blocks reject duplicate spends and are validated (hash, difficulty, transaction ids) when received from peers.
- Merkle root is computed once per mining run instead of on every nonce attempt.
- `send` now mines a block and updates the index, or submits to a node with `--node`.
- Network frames are a plain bincode `Message` enum; the fixed-width command prefix was dropped.
- New `node start` command with an accept loop, handshake, block sync and optional mining.
- Documentation restructured with diagrams and terminal screenshots.

## 0.1.0

Initial experimental version.
