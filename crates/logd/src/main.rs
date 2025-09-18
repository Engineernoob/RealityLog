use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Context;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use reality_core::{
    leaf_hash, make_proof, root as merkle_root, AnchorRecord, AppendRequest, AppendResponse, InclusionProof,
    MerkleError, RootResponse, VerifyRequest, VerifyResponse,
};
use tokio::sync::RwLock;
use tracing::{error, info};

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
struct LogEntry {
    payload: String,
    leaf: String,
    appended_at: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
struct StateSnapshot {
    leaves: Vec<String>,
    entries: Vec<LogEntry>,
}

#[derive(Clone)]
struct AppState {
    inner: Arc<RwLock<StateSnapshot>>,
    data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let data_dir = PathBuf::from(env::var("REALITY_LOG_DIR").unwrap_or_else(|_| "data".to_string()));
    let state = AppState::new(data_dir).await?;

    let app = Router::new()
        .route("/health", get(health))
        .route("/append", post(append))
        .route("/root", get(root))
        .route("/prove/:index", get(prove))
        .route("/verify", post(verify))
        .route("/anchors", get(anchors))
        .with_state(state.clone());

    info!("listening", %addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

impl AppState {
    async fn new(data_dir: PathBuf) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(&data_dir)
            .await
            .context("create data dir")?;

        let leaves: Vec<String> = read_json(data_dir.join("leaves.json")).await?.unwrap_or_default();
        let entries: Vec<LogEntry> = read_json(data_dir.join("entries.json")).await?.unwrap_or_default();

        ensure_file(data_dir.join("anchors.json")).await?;

        Ok(Self {
            inner: Arc::new(RwLock::new(StateSnapshot { leaves, entries })),
            data_dir,
        })
    }

    async fn persist(&self, snapshot: &StateSnapshot) -> anyhow::Result<()> {
        write_json(self.data_path("leaves.json"), &snapshot.leaves).await?;
        write_json(self.data_path("entries.json"), &snapshot.entries).await?;
        Ok(())
    }

    async fn read_anchors(&self) -> anyhow::Result<Vec<AnchorRecord>> {
        Ok(read_json(self.data_path("anchors.json")).await?.unwrap_or_default())
    }

    fn data_path(&self, name: &str) -> PathBuf {
        self.data_dir.join(name)
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn append(
    State(state): State<AppState>,
    Json(req): Json<AppendRequest>,
) -> Result<Json<AppendResponse>, (StatusCode, String)> {
    let leaf_bytes = leaf_hash(req.payload.as_bytes());
    let leaf_hex = hex::encode(leaf_bytes);
    let entry = LogEntry {
        payload: req.payload.clone(),
        leaf: leaf_hex.clone(),
        appended_at: Utc::now().to_rfc3339(),
    };

    let (response, snapshot) = {
        let mut guard = state.inner.write().await;
        guard.leaves.push(leaf_hex.clone());
        guard.entries.push(entry);
        let snapshot = guard.clone();
        let index = snapshot.leaves.len() as u64 - 1;
        let leaves = match decode_leaves(&snapshot.leaves) {
            Ok(l) => l,
            Err(e) => {
                error!(?e, "failed to decode leaves");
                return Err((StatusCode::INTERNAL_SERVER_ERROR, "corrupt leaf storage".into()));
            }
        };
        let root_hex = hex::encode(merkle_root(&leaves));
        (
            AppendResponse {
                index,
                size: snapshot.leaves.len() as u64,
                leaf: leaf_hex,
                root: root_hex,
            },
            snapshot,
        )
    };

    if let Err(err) = state.persist(&snapshot).await {
        error!(?err, "persist failure");
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "persist failure".into()));
    }

    Ok(Json(response))
}

async fn root(State(state): State<AppState>) -> Result<Json<RootResponse>, (StatusCode, String)> {
    let snapshot = state.inner.read().await.clone();
    let leaves = match decode_leaves(&snapshot.leaves) {
        Ok(l) => l,
        Err(e) => {
            error!(?e, "failed to decode leaves");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "corrupt leaf storage".into()));
        }
    };
    let root_hex = hex::encode(merkle_root(&leaves));
    Ok(Json(RootResponse {
        root: root_hex,
        size: snapshot.leaves.len() as u64,
    }))
}

async fn prove(
    Path(index): Path<usize>,
    State(state): State<AppState>,
) -> Result<Json<InclusionProof>, (StatusCode, String)> {
    let snapshot = state.inner.read().await.clone();
    let leaves = match decode_leaves(&snapshot.leaves) {
        Ok(l) => l,
        Err(e) => {
            error!(?e, "failed to decode leaves");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "corrupt leaf storage".into()));
        }
    };

    let proof = make_proof(&leaves, index).map_err(|err| match err {
        MerkleError::IndexOutOfRange => (StatusCode::NOT_FOUND, "leaf index out of range".into()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "unable to build proof".into()),
    })?;

    Ok(Json(proof))
}

async fn verify(
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    Ok(Json(reality_core::verify(&req)))
}

async fn anchors(State(state): State<AppState>) -> Result<Json<Vec<AnchorRecord>>, (StatusCode, String)> {
    match state.read_anchors().await {
        Ok(records) => Ok(Json(records)),
        Err(err) => {
            error!(?err, "failed to read anchors");
            Err((StatusCode::INTERNAL_SERVER_ERROR, "failed to read anchors".into()))
        }
    }
}

fn decode_leaves(hashes: &[String]) -> Result<Vec<[u8; 32]>, hex::FromHexError> {
    hashes.iter().map(|h| decode_hash(h)).collect()
}

fn decode_hash(hex_str: &str) -> Result<[u8; 32], hex::FromHexError> {
    let bytes = hex::decode(hex_str)?;
    if bytes.len() != 32 {
        return Err(hex::FromHexError::InvalidStringLength);
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

async fn read_json<T>(path: PathBuf) -> anyhow::Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => {
            if content.trim().is_empty() {
                return Ok(None);
            }
            let value = serde_json::from_str(&content)?;
            Ok(Some(value))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

async fn write_json<T>(path: PathBuf, value: &T) -> anyhow::Result<()>
where
    T: serde::Serialize,
{
    let json = serde_json::to_string_pretty(value)?;
    tokio::fs::write(path, json).await?;
    Ok(())
}

async fn ensure_file(path: PathBuf) -> anyhow::Result<()> {
    if tokio::fs::metadata(&path).await.is_err() {
        tokio::fs::write(path, b"[]").await?;
    }
    Ok(())
}
