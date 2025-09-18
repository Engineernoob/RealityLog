use sha2::{Digest, Sha256};

use crate::types::{InclusionProof, MerkleError, VerifyRequest, VerifyResponse};

/// Hash a leaf (payload) into a 32-byte digest.
pub fn leaf_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x00]); // domain-separation prefix
    hasher.update(data);
    hasher.finalize().into()
}

/// Hash two nodes together into a parent.
fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x01]); // domain-separation prefix
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Compute the Merkle root from a slice of leaves.
pub fn root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return Sha256::digest(b"EMPTY").into();
    }
    let mut level: Vec<[u8; 32]> = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
        for pair in level.chunks(2) {
            let h = match pair {
                [a, b] => node_hash(a, b),
                [a] => node_hash(a, a),
                _ => unreachable!(),
            };
            next.push(h);
        }
        level = next;
    }
    level[0]
}

/// Build an inclusion proof for a given leaf index.
pub fn make_proof(leaves: &[[u8; 32]], index: usize) -> Result<InclusionProof, MerkleError> {
    if index >= leaves.len() {
        return Err(MerkleError::IndexOutOfRange);
    }

    let mut siblings = Vec::new();
    let mut idx = index;
    let mut level: Vec<[u8; 32]> = leaves.to_vec();

    while level.len() > 1 {
        let sib_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
        let sib = level.get(sib_idx).cloned().unwrap_or(level[idx]);
        siblings.push(hex::encode(sib));

        // move up
        idx /= 2;

        // build next level
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
        for pair in level.chunks(2) {
            let h = match pair {
                [a, b] => node_hash(a, b),
                [a] => node_hash(a, a),
                _ => unreachable!(),
            };
            next.push(h);
        }
        level = next;
    }

    Ok(InclusionProof {
        leaf: hex::encode(leaves[index]),
        index: index as u64,
        siblings,
        root: hex::encode(level[0]),
    })
}

/// Verify a proof request.
pub fn verify(req: &VerifyRequest) -> VerifyResponse {
    let mut acc = match hex::decode(&req.leaf) {
        Ok(bytes) => {
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                arr
            } else {
                return VerifyResponse { valid: false };
            }
        }
        Err(_) => return VerifyResponse { valid: false },
    };

    let mut idx = req.index as usize;
    for sib_hex in &req.siblings {
        let sib_bytes = match hex::decode(sib_hex) {
            Ok(bytes) => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                arr
            }
            Err(_) => return VerifyResponse { valid: false },
        };
        if idx % 2 == 0 {
            acc = node_hash(&acc, &sib_bytes);
        } else {
            acc = node_hash(&sib_bytes, &acc);
        }
        idx /= 2;
    }

    VerifyResponse {
        valid: hex::encode(acc) == req.root,
    }
}