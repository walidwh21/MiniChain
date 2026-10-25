# Network protocol

## Framing

Each message is one TCP connection:

1. the sender connects,
2. writes the bincode encoding of a `Message`,
3. closes the connection.

The receiver reads until end of stream (capped at 16 MiB) and decodes the
payload. There are no replies on the same connection; answers are separate
messages sent to the `from` address contained in the request.

## Messages

| Variant | Payload | Meaning |
| --- | --- | --- |
| `Hello` | `from`, `version`, `height` | Handshake and chain height announcement |
| `Peers` | list of addresses | Known peers |
| `Inventory` | `from`, `kind` (`Block` or `Tx`), `ids` | "I have these items" |
| `GetItem` | `from`, `kind`, `id` | Request a single block or transaction |
| `GetBlocks` | `from` | Request all block hashes |
| `Block` | `from`, `block` | A full block |
| `Tx` | `from`, `tx` | A full transaction |

`from` is the `host:port` the sender listens on. A client that does not listen
(for example `send --node`) uses an empty string.

## Flows

### Handshake

On receiving `Hello`, a node compares heights:

- the sender is ahead -> reply with `GetBlocks`;
- the sender is behind -> reply with our own `Hello`;
- in both cases send `Peers` and remember the sender.

A node that is not the seed introduces itself to the seed on start-up.

### Block download

1. `GetBlocks` is answered with `Inventory(Block, hashes)`, newest first.
2. The receiver drops hashes it already has, reverses the list so the oldest is
   first, requests the first with `GetItem` and queues the rest.
3. Every arriving `Block` is validated and stored, then the next queued hash is
   requested. When the queue is empty the UTXO index is rebuilt.

### Transaction relay and mining

1. A client submits `Tx` to a node.
2. The seed node only relays: it announces the id to all other peers with
   `Inventory(Tx, ...)`, and peers fetch the body with `GetItem`.
3. A non-seed node started with `--miner` verifies the mempool, discards
   invalid or already spent transactions, mines the rest together with a
   coinbase, updates its UTXO index and announces the block hash to its peers.

## Security notes

The protocol has no authentication, encryption, peer scoring or rate limiting.
It is intended for local experiments only.
