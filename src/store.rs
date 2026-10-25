//! Persistent storage.
//!
//! A single embedded `sled` database holds every namespace as its own tree,
//! so the whole process shares one file lock.

use crate::config;
use crate::error::Result;

#[derive(Clone)]
pub struct Store {
    db: sled::Db,
}

impl Store {
    /// Opens (or creates) the database under the configured data directory.
    pub fn open() -> Result<Store> {
        let path = format!("{}/store", config::data_dir());
        Store::open_at(&path)
    }

    pub fn open_at(path: &str) -> Result<Store> {
        Ok(Store {
            db: sled::open(path)?,
        })
    }

    /// Block hash -> serialized block.
    pub fn blocks(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("blocks")?)
    }

    /// Small bookkeeping values such as the chain head.
    pub fn meta(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("meta")?)
    }

    /// Transaction id -> unspent output slots.
    pub fn utxo(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("utxo")?)
    }

    /// Address -> serialized key pair.
    pub fn keys(&self) -> Result<sled::Tree> {
        Ok(self.db.open_tree("keys")?)
    }

    /// In-memory database that disappears on drop; used by tests.
    #[cfg(test)]
    pub fn temporary() -> Result<Store> {
        Ok(Store {
            db: sled::Config::new().temporary(true).open()?,
        })
    }
}
