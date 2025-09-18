pub mod types;
use sha2::{Digest, Sha256};
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const LEAF_PREFIX: [u8; 1] = [0x00];
const NODE_PREFIX: [u8; 1] = [0x01];
const EMPTY_SENTINEL: &[u8] = b"EMPTY";

#[derive(Debug, Error)]
pub enum MerkleError {
    #[error("index out of range")]
    IndexOutOfRange,
    #[error("invalid hex string")]
    InvalidHex,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    fn as_str(&self) -> &'static str {
        match self {
            Direction::Left => "left",
            Direction::Right => "right",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofStep {
    pub direction: Direction,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InclusionProof {
    pub index: u64,
    pub leaf: String,
    pub path: Vec<ProofStep>,
    pub root: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppendRequest {
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppendResponse {
    pub index: u64,
    pub size: u64,
    pub leaf: String,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RootResponse {
    pub root: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifyRequest {
    pub index: u64,
    pub leaf: String,
    pub path: Vec<ProofStep>,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifyResponse {
    pub valid: bool,
    pub computed_root: String,
    pub expected_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnchorRecord {
    pub root: String,
    pub size: u64,
    pub timestamp_nanos: String,
    pub txid: String,
}

pub fn leaf_hash(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(LEAF_PREFIX);
    hasher.update(bytes);
    hasher.finalize().into()
}

pub fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(NODE_PREFIX);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

pub fn empty_root() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(EMPTY_SENTINEL);
    hasher.finalize().into()
}

pub fn root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return empty_root();
    }

    let mut layer: Vec<[u8; 32]> = leaves.to_vec();
    while layer.len() > 1 {
        layer = parents(&layer);
    }
    layer[0]
}

fn parents(layer: &[[u8; 32]]) -> Vec<[u8; 32]> {
    let mut parents = Vec::with_capacity((layer.len() + 1) / 2);
    let mut iter = layer.chunks(2);
    while let Some(chunk) = iter.next() {
        let left = chunk[0];
        let right = if chunk.len() == 2 { chunk[1] } else { chunk[0] };
        parents.push(node_hash(&left, &right));
    }
    parents
}

pub fn inclusion_path(leaves: &[[u8; 32]], index: usize) -> Result<Vec<ProofStep>, MerkleError> {
    if index >= leaves.len() {
        return Err(MerkleError::IndexOutOfRange);
    }

    if leaves.len() <= 1 {
        return Ok(Vec::new());
    }

    let mut path = Vec::new();
    let mut idx = index;
    let mut layer: Vec<[u8; 32]> = leaves.to_vec();

    while layer.len() > 1 {
        let is_right = idx % 2 == 1;
        let sibling_idx = if is_right {
            idx - 1
        } else if idx + 1 < layer.len() {
            idx + 1
        } else {
            idx
        };

        let sibling_hash = layer[sibling_idx];
        let direction = if is_right {
            Direction::Left
        } else {
            Direction::Right
        };

        path.push(ProofStep {
            direction,
            hash: hex::encode(sibling_hash),
        });

        layer = parents(&layer);
        idx /= 2;
    }

    Ok(path)
}

pub fn make_proof(leaves: &[[u8; 32]], index: usize) -> Result<InclusionProof, MerkleError> {
    let size = leaves.len() as u64;
    let leaf = leaves
        .get(index)
        .ok_or(MerkleError::IndexOutOfRange)?
        .to_owned();
    let path = inclusion_path(leaves, index)?;
    let root = hex::encode(root(leaves));

    Ok(InclusionProof {
        index: index as u64,
        leaf: hex::encode(leaf),
        path,
        root,
        size,
    })
}

pub fn verify(req: &VerifyRequest) -> VerifyResponse {
    let expected_root = normalize_hex(&req.root);

    let mut computed = match decode_hash(&req.leaf) {
        Some(bytes) => bytes,
        None => {
            return VerifyResponse {
                valid: false,
                computed_root: String::new(),
                expected_root,
            }
        }
    };

    for step in &req.path {
        let sibling = match decode_hash(&step.hash) {
            Some(bytes) => bytes,
            None => {
                return VerifyResponse {
                    valid: false,
                    computed_root: String::new(),
                    expected_root,
                };
            }
        };

        computed = match step.direction {
            Direction::Left => node_hash(&sibling, &computed),
            Direction::Right => node_hash(&computed, &sibling),
        };
    }

    let computed_root = hex::encode(computed);
    let valid = computed_root == expected_root;

    VerifyResponse {
        valid,
        computed_root,
        expected_root,
    }
}

fn decode_hash(hex_str: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(hex_str).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Some(array)
}

fn normalize_hex(value: &str) -> String {
    value.to_ascii_lowercase()
}

impl fmt::Display for ProofStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.direction.as_str(), self.hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(data: &str) -> [u8; 32] {
        leaf_hash(data.as_bytes())
    }

    #[test]
    fn leaf_hash_is_deterministic() {
        let a = h("hello");
        let b = h("hello");
        assert_eq!(a, b);
        assert_ne!(a, h("world"));
    }

    #[test]
    fn merkle_root_matches_known_values() {
        let leaves1 = vec![h("a")];
        let leaves2 = vec![h("a"), h("b")];
        let leaves3 = vec![h("a"), h("b"), h("c")];
        let leaves4 = vec![h("a"), h("b"), h("c"), h("d")];

        let root1 = hex::encode(root(&leaves1));
        let root2 = hex::encode(root(&leaves2));
        let root3 = hex::encode(root(&leaves3));
        let root4 = hex::encode(root(&leaves4));

        assert_eq!(
            root1,
            "022a6979e6dab7aa5ae4c3e5e45f7e977112a7e63593820dbec1ec738a24f93c"
        );
        assert_eq!(
            root2,
            "b137985ff484fb600db93107c77b0365c80d78f5b429ded0fd97361d077999eb"
        );
        assert_eq!(
            root3,
            "e9636069c740c9ff51625b01a0b040396d265a9b920cc6febdfa5ecc9f58ecce"
        );
        assert_eq!(
            root4,
            "33376a3bd63e9993708a84ddfe6c28ae58b83505dd1fed711bd924ec5a6239f0"
        );
    }

    #[test]
    fn inclusion_proof_round_trip() {
        let leaves = vec![h("alpha"), h("beta"), h("gamma"), h("delta")];
        let proof = make_proof(&leaves, 2).expect("proof");
        let verify_req = VerifyRequest {
            index: proof.index,
            leaf: proof.leaf.clone(),
            path: proof.path.clone(),
            root: proof.root.clone(),
        };

        let response = verify(&verify_req);
        assert!(response.valid);
        assert_eq!(response.expected_root, proof.root);
    }
}
