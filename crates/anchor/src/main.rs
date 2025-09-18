use std::{env, path::PathBuf, time::Duration};

use anyhow::Context;
use reality_core::{AnchorRecord, RootResponse};
use reqwest::Client;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tokio::time::sleep;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let api = env::var("REALITY_LOG_API").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let data_dir =
        PathBuf::from(env::var("REALITY_LOG_DIR").unwrap_or_else(|_| "data".to_string()));
    tokio::fs::create_dir_all(&data_dir)
        .await
        .context("create data dir")?;
    let anchors_path = data_dir.join("anchors.json");

    let mut anchors: Vec<AnchorRecord> = read_json(&anchors_path).await?.unwrap_or_default();
    if anchors.is_empty() {
        ensure_file(&anchors_path).await?;
    }
    let mut last_anchor = anchors.last().cloned();

    let client = Client::builder().build()?;

    loop {
        match fetch_root(&client, &api).await {
            Ok(root) => {
                let is_new = last_anchor
                    .as_ref()
                    .map(|a| a.root != root.root || a.size != root.size)
                    .unwrap_or(true);

                if is_new {
                    let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos().to_string();
                    let txid = compute_txid(root.size, &root.root, &timestamp);
                    let record = AnchorRecord {
                        root: root.root.clone(),
                        size: root.size,
                        timestamp_nanos: timestamp,
                        txid,
                    };
                    anchors.push(record.clone());
                    write_json(&anchors_path, &anchors).await?;
                    last_anchor = Some(record.clone());
                    info!(
                        root = %record.root,
                        size = record.size,
                        txid = %record.txid,
                        "anchored new root"
                    );
                }
            }
            Err(err) => {
                warn!(?err, "failed to fetch root");
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}

async fn fetch_root(client: &Client, base: &str) -> anyhow::Result<RootResponse> {
    let url = format!("{}/root", base.trim_end_matches('/'));
    let resp = client.get(url).send().await?.error_for_status()?;
    Ok(resp.json::<RootResponse>().await?)
}

fn compute_txid(size: u64, root: &str, timestamp: &str) -> String {
    let payload = format!("{}:{}:{}", size, root, timestamp);
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    let digest: [u8; 32] = hasher.finalize().into();
    hex::encode(digest)
}

async fn read_json<T>(path: &PathBuf) -> anyhow::Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    match tokio::fs::read_to_string(path).await {
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

async fn write_json<T>(path: &PathBuf, value: &T) -> anyhow::Result<()>
where
    T: serde::Serialize,
{
    let json = serde_json::to_string_pretty(value)?;
    tokio::fs::write(path, json).await?;
    Ok(())
}

async fn ensure_file(path: &PathBuf) -> anyhow::Result<()> {
    if tokio::fs::metadata(path).await.is_err() {
        tokio::fs::write(path, b"[]").await?;
    }
    Ok(())
}
