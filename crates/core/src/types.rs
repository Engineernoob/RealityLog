use serde::{Deserialize, Serialize};

/// Represents the Merkle root of the log at a given size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootResponse {
    /// Total number of leaves in the log.
    pub size: u64,
    /// Hex-encoded Merkle root.
    pub root: String,
}

/// Record of an anchor event — when a Merkle root was checkpointed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorRecord {
    /// The Merkle root that was anchored.
    pub root: String,
    /// The size of the log at the time of anchoring.
    pub size: u64,
    /// Nanosecond-resolution timestamp when anchored (string for now).
    pub timestamp_nanos: String,
    /// Transaction ID or simulated hash tying this anchor to an external system.
    pub txid: String,
}

/// Append request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendRequest {
    pub payload: String,
}

/// Response after appending a new leaf.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendResponse {
    pub index: u64,
    pub size: u64,
    pub leaf: String,
    pub root: String,
}

/// Merkle inclusion proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InclusionProof {
    pub leaf: String,
    pub index: u64,
    pub siblings: Vec<String>,
    pub root: String,
}

/// Request to verify a proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub leaf: String,
    pub index: u64,
    pub siblings: Vec<String>,
    pub root: String,
}

/// Response to a proof verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub valid: bool,
}

/// Errors from the Merkle module.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MerkleError {
    #[error("leaf index out of range")]
    IndexOutOfRange,
    #[error("invalid proof")]
    InvalidProof,
}

