#![no_std]

use elrond_wasm::types::{BoxedBytes, H256};
use zero_copy_source::ZeroCopySource;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, PartialEq)]
pub enum MerkleProofNodePosition {
    Left,
    Right,
}

impl MerkleProofNodePosition {
    fn as_u8(&self) -> u8 {
        match *self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }

    fn from_u8(val: u8) -> SCResult<Self> {
        match val {
            0 => Ok(Self::Left),
            1 => Ok(Self::Right),
            _ => sc_error!("Failed From<u8> for MerkleProofNodePosition"),
        }
    }
}

#[derive(TypeAbi)]
pub struct MerkleProofNode {
    pub position: MerkleProofNodePosition,
    pub hash: H256,
}

pub struct MerkleProof<CA>
where
    CA: CryptoApi,
{
    api: CA,
    raw_leaf: BoxedBytes,
    nodes: Vec<MerkleProofNode>,
}

impl<CA: CryptoApi> MerkleProof<CA> {
    pub fn from_bytes(api: CA, proof_bytes: &BoxedBytes) -> SCResult<Self> {
        let mut source = ZeroCopySource::new(proof_bytes.as_slice());
        let raw_leaf;
        let mut nodes = Vec::new();

        match source.next_var_bytes() {
            Some(leaf) => raw_leaf = leaf,
            None => return sc_error!("Merkle Proof deserialization failed: Raw leaf"),
        }

        while source.get_bytes_left() > 0 {
            let position;
            let hash;

            match source.next_u8() {
                Some(pos) => position = MerkleProofNodePosition::from_u8(pos)?,
                None => return sc_error!("Merkle Proof deserialization failed: Pos"),
            }
            match source.next_hash() {
                Some(h) => hash = h,
                None => return sc_error!("Merkle Proof deserialization failed: Hash"),
            }

            nodes.push(MerkleProofNode { hash, position });
        }

        Ok(Self {
            api,
            nodes,
            raw_leaf,
        })
    }
}

impl<CA: CryptoApi> MerkleProof<CA> {
    pub fn get_proof_root(&self) -> H256 {
        let mut current_hash = self.hash_leaf(&self.raw_leaf);

        for node in &self.nodes {
            match node.position {
                MerkleProofNodePosition::Left => {
                    current_hash = self.hash_children(&node.hash, &current_hash);
                }
                MerkleProofNodePosition::Right => {
                    current_hash = self.hash_children(&current_hash, &node.hash);
                }
            }
        }

        current_hash
    }

    pub fn into_raw_leaf(self) -> BoxedBytes {
        self.raw_leaf
    }

    fn hash_leaf(&self, raw_leaf: &BoxedBytes) -> H256 {
        let mut serialized = Vec::with_capacity(1 + raw_leaf.len());
        serialized.push(MerkleProofNodePosition::Left.as_u8());
        serialized.extend_from_slice(raw_leaf.as_slice());

        self.api.sha256(&serialized)
    }

    fn hash_children(&self, left: &H256, right: &H256) -> H256 {
        let mut serialized = Vec::with_capacity(1 + 2 * H256::len_bytes());
        serialized.push(MerkleProofNodePosition::Right.as_u8());
        serialized.extend_from_slice(left.as_bytes());
        serialized.extend_from_slice(right.as_bytes());

        self.api.sha256(&serialized)
    }
}
