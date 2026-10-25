//! A running node: shared state, the accept loop and outbound helpers.

use super::message::{decode, encode, GetBlocks, Hello, Inventory, ItemKind, Message};
use crate::chain::{Ledger, Transaction, UtxoSet};
use crate::config::{MAX_MESSAGE_BYTES, PROTOCOL_VERSION, SEED_NODE};
use crate::error::Result;
use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

/// Mutable node state shared between connection threads.
pub(super) struct State {
    /// Addresses of peers we may talk to.
    pub peers: HashSet<String>,
    /// Block hashes still to be downloaded during a sync, oldest first.
    pub pending_blocks: Vec<String>,
    /// Transactions waiting to be mined.
    pub mempool: HashMap<String, Transaction>,
}

#[derive(Clone)]
pub struct Peer {
    pub(super) addr: String,
    pub(super) miner: Option<String>,
    pub(super) ledger: Ledger,
    pub(super) utxo: UtxoSet,
    state: Arc<Mutex<State>>,
}

impl Peer {
    /// `miner` is the address that receives block rewards; `None` makes the
    /// node a plain relay.
    pub fn new(addr: String, miner: Option<String>, ledger: Ledger, utxo: UtxoSet) -> Peer {
        let mut peers = HashSet::new();
        peers.insert(SEED_NODE.to_string());
        Peer {
            addr,
            miner,
            ledger,
            utxo,
            state: Arc::new(Mutex::new(State {
                peers,
                pending_blocks: Vec::new(),
                mempool: HashMap::new(),
            })),
        }
    }

    /// Binds the listening socket and serves connections forever, one
    /// thread per connection.
    pub fn serve(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.addr)?;
        info!("listening on {}", self.addr);
        if self.addr != SEED_NODE {
            self.send_hello(SEED_NODE)?;
        }
        for incoming in listener.incoming() {
            match incoming {
                Ok(stream) => {
                    let worker = self.clone();
                    thread::spawn(move || {
                        if let Err(problem) = worker.serve_connection(stream) {
                            error!("connection failed: {}", problem);
                        }
                    });
                }
                Err(problem) => warn!("accept failed: {}", problem),
            }
        }
        Ok(())
    }

    fn serve_connection(&self, mut stream: TcpStream) -> Result<()> {
        let mut buffer = Vec::new();
        (&mut stream)
            .take(MAX_MESSAGE_BYTES)
            .read_to_end(&mut buffer)?;
        let message = decode(&buffer)?;
        debug!("inbound: {:?}", message);
        self.dispatch(message)
    }

    pub(super) fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(super) fn known_peers(&self) -> Vec<String> {
        self.state().peers.iter().cloned().collect()
    }

    pub(super) fn remember_peer(&self, address: &str) {
        if !address.is_empty() && address != self.addr {
            self.state().peers.insert(address.to_string());
        }
    }

    /// Sends one frame. An unreachable peer is forgotten rather than
    /// reported as an error.
    pub(super) fn send(&self, target: &str, message: &Message) -> Result<()> {
        if target == self.addr {
            return Ok(());
        }
        let bytes = encode(message)?;
        match TcpStream::connect(target) {
            Ok(mut stream) => {
                stream.write_all(&bytes)?;
                Ok(())
            }
            Err(_) => {
                warn!("peer {} unreachable, dropping it", target);
                self.state().peers.remove(target);
                Ok(())
            }
        }
    }

    pub(super) fn send_hello(&self, target: &str) -> Result<()> {
        let height = self.ledger.best_height()?;
        self.send(
            target,
            &Message::Hello(Hello {
                from: self.addr.clone(),
                version: PROTOCOL_VERSION,
                height,
            }),
        )
    }

    pub(super) fn send_peers(&self, target: &str) -> Result<()> {
        self.send(target, &Message::Peers(self.known_peers()))
    }

    pub(super) fn send_get_blocks(&self, target: &str) -> Result<()> {
        self.send(
            target,
            &Message::GetBlocks(GetBlocks {
                from: self.addr.clone(),
            }),
        )
    }

    pub(super) fn send_inventory(
        &self,
        target: &str,
        kind: ItemKind,
        ids: Vec<String>,
    ) -> Result<()> {
        self.send(
            target,
            &Message::Inventory(Inventory {
                from: self.addr.clone(),
                kind,
                ids,
            }),
        )
    }
}
