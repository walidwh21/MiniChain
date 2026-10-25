//! Reactions to inbound messages, plus the mining trigger.

use super::message::{
    BlockMsg, GetBlocks, GetItem, Hello, Inventory, ItemKind, Message, TxMsg,
};
use super::peer::Peer;
use crate::chain::Transaction;
use crate::config::SEED_NODE;
use crate::error::Result;
use log::info;

impl Peer {
    pub(super) fn dispatch(&self, message: Message) -> Result<()> {
        match message {
            Message::Hello(m) => self.on_hello(m),
            Message::Peers(list) => self.on_peers(list),
            Message::Inventory(m) => self.on_inventory(m),
            Message::GetItem(m) => self.on_get_item(m),
            Message::GetBlocks(m) => self.on_get_blocks(m),
            Message::Block(m) => self.on_block(m),
            Message::Tx(m) => self.on_tx(m),
        }
    }

    /// Compare heights: pull blocks if the sender is ahead, push our own
    /// hello if we are ahead, then share the peer list.
    fn on_hello(&self, msg: Hello) -> Result<()> {
        info!("hello from {} (height {})", msg.from, msg.height);
        let ours = self.ledger.best_height()?;
        if ours < msg.height {
            self.send_get_blocks(&msg.from)?;
        } else if ours > msg.height {
            self.send_hello(&msg.from)?;
        }
        self.send_peers(&msg.from)?;
        self.remember_peer(&msg.from);
        Ok(())
    }

    fn on_peers(&self, list: Vec<String>) -> Result<()> {
        for address in list {
            self.remember_peer(&address);
        }
        Ok(())
    }

    fn on_get_blocks(&self, msg: GetBlocks) -> Result<()> {
        let hashes = self.ledger.block_hashes();
        self.send_inventory(&msg.from, ItemKind::Block, hashes)
    }

    fn on_inventory(&self, msg: Inventory) -> Result<()> {
        match msg.kind {
            ItemKind::Block => {
                // Announcements list newest first; download oldest first so
                // every block arrives after its parent.
                let mut wanted = Vec::new();
                for id in msg.ids.iter().rev() {
                    if !self.ledger.has_block(id)? {
                        wanted.push(id.clone());
                    }
                }
                if wanted.is_empty() {
                    return Ok(());
                }
                let first = wanted.remove(0);
                self.state().pending_blocks = wanted;
                self.send(
                    &msg.from,
                    &Message::GetItem(GetItem {
                        from: self.addr.clone(),
                        kind: ItemKind::Block,
                        id: first,
                    }),
                )
            }
            ItemKind::Tx => {
                for id in msg.ids {
                    let known = self.state().mempool.contains_key(&id);
                    if !known {
                        self.send(
                            &msg.from,
                            &Message::GetItem(GetItem {
                                from: self.addr.clone(),
                                kind: ItemKind::Tx,
                                id,
                            }),
                        )?;
                    }
                }
                Ok(())
            }
        }
    }

    fn on_get_item(&self, msg: GetItem) -> Result<()> {
        match msg.kind {
            ItemKind::Block => {
                if let Some(block) = self.ledger.get_block(&msg.id)? {
                    self.send(
                        &msg.from,
                        &Message::Block(BlockMsg {
                            from: self.addr.clone(),
                            block,
                        }),
                    )?;
                }
            }
            ItemKind::Tx => {
                let found = self.state().mempool.get(&msg.id).cloned();
                if let Some(tx) = found {
                    self.send(
                        &msg.from,
                        &Message::Tx(TxMsg {
                            from: self.addr.clone(),
                            tx,
                        }),
                    )?;
                }
            }
        }
        Ok(())
    }

    fn on_block(&self, msg: BlockMsg) -> Result<()> {
        info!("block {} from {}", msg.block.hash(), msg.from);
        self.ledger.append_block(msg.block)?;

        let next = {
            let mut state = self.state();
            if state.pending_blocks.is_empty() {
                None
            } else {
                Some(state.pending_blocks.remove(0))
            }
        };
        match next {
            Some(hash) => self.send(
                &msg.from,
                &Message::GetItem(GetItem {
                    from: self.addr.clone(),
                    kind: ItemKind::Block,
                    id: hash,
                }),
            ),
            None => {
                self.utxo.rebuild()?;
                Ok(())
            }
        }
    }

    /// The seed node only relays; any other node with a miner address mines.
    fn on_tx(&self, msg: TxMsg) -> Result<()> {
        info!("transaction {} from {:?}", msg.tx.id, msg.from);
        self.state()
            .mempool
            .insert(msg.tx.id.clone(), msg.tx.clone());

        if self.addr == SEED_NODE {
            for peer in self.known_peers() {
                if peer != self.addr && peer != msg.from {
                    self.send_inventory(&peer, ItemKind::Tx, vec![msg.tx.id.clone()])?;
                }
            }
            Ok(())
        } else {
            self.try_mine()
        }
    }

    /// Mines every valid, still-spendable transaction in the mempool into one
    /// block and announces it.
    fn try_mine(&self) -> Result<()> {
        let miner = match self.miner.clone() {
            Some(address) => address,
            None => return Ok(()),
        };
        let candidates: Vec<Transaction> = self.state().mempool.values().cloned().collect();
        if candidates.is_empty() {
            return Ok(());
        }

        let mut batch = Vec::new();
        let mut rejected = Vec::new();
        for tx in candidates {
            let acceptable = self.ledger.verify_tx(&tx)? && self.utxo.can_spend(&tx)?;
            if acceptable {
                batch.push(tx);
            } else {
                rejected.push(tx.id.clone());
            }
        }
        {
            let mut state = self.state();
            for id in &rejected {
                state.mempool.remove(id);
            }
        }
        if batch.is_empty() {
            return Ok(());
        }

        let mined_ids: Vec<String> = batch.iter().map(|tx| tx.id.clone()).collect();
        let mut transactions = vec![Transaction::coinbase(&miner, "")?];
        transactions.extend(batch);

        let block = self.ledger.mine_block(transactions)?;
        self.utxo.apply_block(&block)?;
        {
            let mut state = self.state();
            for id in &mined_ids {
                state.mempool.remove(id);
            }
        }

        for peer in self.known_peers() {
            if peer != self.addr {
                self.send_inventory(&peer, ItemKind::Block, vec![block.hash().to_string()])?;
            }
        }
        Ok(())
    }
}
