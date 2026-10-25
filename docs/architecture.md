# Architecture

## Module responsibilities

| Module | Responsibility |
| --- | --- |
| `crypto` | SHA-256, hash160 (RIPEMD-160 over SHA-256), ed25519 sign / verify. No other module touches a crypto crate directly. |
| `error` | `Result<T>` and a boxed `Error` so `?` works everywhere; `err("message")` builds an error from text. |
| `config` | Protocol constants (difficulty, reward, seed node) and the `MINICHAIN_DATA` lookup. |
| `store` | Opens one `sled` database and hands out named trees. |
| `wallet` | `Keypair`, address encode / decode, `Keystore` (persistent key pairs). |
| `chain::tx` | `TxIn` and `TxOut`. |
| `chain::transaction` | Building payments and coinbases, signing digests, full verification. |
| `chain::merkle` | Merkle root over transaction ids using SHA-256 as the merge function. |
| `chain::block` | Block structure, nonce search, structural validity. |
| `chain::ledger` | Block storage, chain head, iterator, mining and appending of blocks. |
| `chain::utxo` | Unspent output index used for balances and coin selection. |
| `net` | Message definitions, node state, accept loop and handlers. |
| `cli` | `clap` command tree and the chain pretty-printer. |

## Storage layout

All data lives in `<data dir>/store`, a single `sled` database.

| Tree | Key | Value |
| --- | --- | --- |
| `blocks` | block hash (hex) | bincode `Block` |
| `meta` | `head` | hash of the newest block |
| `utxo` | transaction id | bincode list of `Option<TxOut>` slots |
| `keys` | address | bincode `Keypair` |

Because one `sled::Db` is shared, tree handles are cheap to clone and every
component sees the same data without extra locking.

## Data flow of a local payment

1. `Keystore::lookup` returns the sender's key pair.
2. `UtxoSet::collect_spendable` picks outputs that cover the amount.
3. `Transaction::new_payment` builds inputs, the payment output and a change
   output, then asks `Ledger::sign_tx` to sign each input.
4. `Ledger::mine_block` verifies every transaction, rejects two spends of the
   same output, searches for a nonce and stores the block.
5. `UtxoSet::apply_block` marks spent slots as `None` and registers new outputs.

## Transaction validity rules

A non-coinbase transaction is valid when all of these hold:

- its `id` equals the SHA-256 of its contents with the id blanked;
- every input references an existing output;
- the hash of the input's public key equals the key hash the output is locked to;
- the signature verifies over that input's signing digest;
- the sum of the referenced outputs is at least the sum of the new outputs.

A coinbase transaction is valid when its id is genuine and it mints exactly
`BLOCK_REWARD` coins. Additionally, a block is only accepted when its hash
matches its contents, meets the difficulty and contains genuine transaction ids.

## Concurrency model

`net::Peer` is `Clone`. Clones share the database handles and an
`Arc<Mutex<State>>` (peer set, pending block queue, mempool). The mutex is only
held for short operations; mining happens on a snapshot of the mempool.
