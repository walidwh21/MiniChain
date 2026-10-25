//! Merkle root over transaction ids.

use crate::crypto;
use merkle_cbt::merkle_tree::{Merge, CBMT};

/// Combines two child hashes into their parent with SHA-256.
pub struct Sha256Merge;

impl Merge for Sha256Merge {
    type Item = Vec<u8>;

    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut joined = Vec::with_capacity(left.len() + right.len());
        joined.extend_from_slice(left);
        joined.extend_from_slice(right);
        crypto::sha256(&joined)
    }
}

/// Root of the complete binary merkle tree built from `leaves`.
pub fn root_of(leaves: &[Vec<u8>]) -> Vec<u8> {
    CBMT::<Vec<u8>, Sha256Merge>::build_merkle_tree(leaves).root()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_changes_with_leaves() {
        let a = root_of(&[b"a".to_vec(), b"b".to_vec()]);
        let b = root_of(&[b"a".to_vec(), b"c".to_vec()]);
        assert_ne!(a, b);
        assert_eq!(a, root_of(&[b"a".to_vec(), b"b".to_vec()]));
    }
}
