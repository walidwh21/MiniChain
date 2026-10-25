# Command line reference

Run `minichain --help` or `minichain <command> --help` for built-in help. When
working from source, replace `minichain` with `cargo run --`.

## wallet

```text
minichain wallet create
minichain wallet list
```

`create` generates an ed25519 key pair, stores it and prints the address.
`list` prints all stored addresses in sorted order.

## chain

```text
minichain chain init <ADDRESS>
minichain chain show
```

`init` validates the address, replaces any existing chain with a fresh genesis
block that pays `BLOCK_REWARD` coins to `ADDRESS`, and builds the UTXO index.
`show` prints every block from the newest to the genesis block.

## balance

```text
minichain balance <ADDRESS>
```

Sums all unspent outputs locked to the address.

## send

```text
minichain send <FROM> <TO> <AMOUNT>
minichain send <FROM> <TO> <AMOUNT> --node <HOST:PORT>
```

`FROM` must have a key in the local keystore. `TO` may be any valid address.

- Without `--node` the transaction is mined into a block immediately; `FROM`
  receives the block reward.
- With `--node` the signed transaction is handed to a running node and nothing
  is mined locally.

## utxo

```text
minichain utxo rebuild
```

Replays the chain and recomputes the unspent-output index. Useful after
copying a data directory or syncing blocks from the network.

## node

```text
minichain node start <PORT> [--miner <ADDRESS>]
```

Listens on `127.0.0.1:PORT`. A node that is not on the seed port introduces
itself to the seed node. With `--miner`, incoming transactions are mined and
rewards are paid to `ADDRESS`.

## Environment

| Variable | Effect |
| --- | --- |
| `MINICHAIN_DATA` | Directory for the database (default `data`) |
| `RUST_LOG` | Log filter, for example `info` or `debug` |
