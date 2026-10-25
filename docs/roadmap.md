# Roadmap

Done:

- [x] Block chain with persistent storage
- [x] Command line interface
- [x] Transactions, genesis block and coinbase rewards
- [x] Key pairs and addresses
- [x] Proof-of-work mining
- [x] Signed inputs with ownership checks
- [x] Merkle root block fingerprint
- [x] Unspent output index
- [x] Peer-to-peer messaging, block sync and mempool relay

Ideas:

- [ ] Split the crate into a library plus separate binaries (node, wallet)
- [ ] Index transactions by id instead of scanning the chain
- [ ] Difficulty adjustment and fork choice by accumulated work
- [ ] Allow spending outputs created earlier in the same block
- [ ] Versioned, documented wire format; consider a different transport (for example gRPC)
- [ ] Persist the mempool and cap its size
- [ ] Bring-your-own genesis file so nodes need not copy data directories
- [ ] Separate light client that talks to a full node
- [ ] Integration tests that spawn several nodes
