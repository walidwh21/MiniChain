//! Persistent collection of key pairs, indexed by address.

use super::Keypair;
use crate::error::Result;
use crate::store::Store;

#[derive(Clone)]
pub struct Keystore {
    tree: sled::Tree,
}

impl Keystore {
    pub fn open(store: &Store) -> Result<Keystore> {
        Ok(Keystore {
            tree: store.keys()?,
        })
    }

    /// Generates a key pair, saves it and returns the new address.
    pub fn create(&self) -> Result<String> {
        let pair = Keypair::generate()?;
        let address = pair.address()?;
        self.tree
            .insert(address.as_bytes(), bincode::serialize(&pair)?)?;
        self.tree.flush()?;
        Ok(address)
    }

    pub fn lookup(&self, address: &str) -> Result<Option<Keypair>> {
        match self.tree.get(address.as_bytes())? {
            Some(raw) => Ok(Some(bincode::deserialize(&raw)?)),
            None => Ok(None),
        }
    }

    /// All stored addresses in sorted order.
    pub fn addresses(&self) -> Result<Vec<String>> {
        let mut found = Vec::new();
        for entry in self.tree.iter() {
            let (key, _) = entry?;
            found.push(String::from_utf8(key.to_vec())?);
        }
        found.sort();
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_keys_can_be_found_again() {
        let store = Store::temporary().unwrap();
        let keys = Keystore::open(&store).unwrap();
        let address = keys.create().unwrap();
        let pair = keys.lookup(&address).unwrap().unwrap();
        assert_eq!(pair.address().unwrap(), address);
        assert_eq!(keys.addresses().unwrap(), vec![address]);
        assert!(keys.lookup("missing").unwrap().is_none());
    }
}
