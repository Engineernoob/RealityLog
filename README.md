# RealityLog Transparency Stack

RealityLog is a minimal content authenticity transparency log built in Rust with a lightweight web verifier. It provides:

- **reality-core**: Merkle tree primitives, proof types, and verification helpers
- **reality-logd**: Axum JSON API with file-backed storage
- **reality-anchor**: Background anchorer that snapshots log roots every 60 seconds
- **reality-wasm-core**: wasm-bindgen wrapper exposing proof verification
- **verifier-ext**: Vite + TypeScript UI that calls the WASM verifier

## Prerequisites

- Rust 1.74+
- wasm-pack (for building the WebAssembly package)
- Node.js 18+ (for the Vite verifier)

## Running the API

```bash
cargo run -p reality-logd
```

The daemon listens on `127.0.0.1:8080` and persists data under `data/` unless `REALITY_LOG_DIR` is set. Health check: `curl http://127.0.0.1:8080/health`.

### Append Entries

```bash
curl -X POST http://127.0.0.1:8080/append \
  -H 'content-type: application/json' \
  -d '{"payload":"hello world"}'
```

### Inspect Roots & Proofs

```bash
curl http://127.0.0.1:8080/root
curl http://127.0.0.1:8080/prove/0
```

### Remote Verification

```bash
curl -X POST http://127.0.0.1:8080/verify \
  -H 'content-type: application/json' \
  -d @proof.json
```

## Anchoring Service

Run the anchorer in a separate terminal:

```bash
REALITY_LOG_API=http://127.0.0.1:8080 \
REALITY_LOG_DIR=data \
cargo run -p reality-anchor
```

Every 60 seconds it fetches the latest root and appends an `AnchorRecord` to `data/anchors.json` using `txid = sha256("{tree_size}:{root}:{timestamp_nanos}")`.

## WebAssembly Verifier

1. Build the WASM package:
   ```bash
   wasm-pack build web/wasm-core --target web --out-dir pkg
   ```
2. Start the Vite app:
   ```bash
   cd web/verifier-ext
   npm install
   npm run dev
   ```
3. Open the served URL, paste a `VerifyRequest` JSON proof, and click **Verify**. Use **Load Sample** for a minimal proof of a single leaf.

## Testing

```bash
cargo test -p reality-core
```

This exercises hash determinism, known Merkle roots for 1–4 leaves, and inclusion proof verification.

## Directory Layout

- `crates/core`: Merkle tree library and shared types
- `crates/logd`: Axum API server with JSON persistence
- `crates/anchor`: Root anchorer loop
- `web/wasm-core`: wasm-bindgen wrapper exposing `verify_inclusion`
- `web/verifier-ext`: Browser verifier UI (expects `web/wasm-core/pkg` build output)
- `data/`: File-backed storage for leaves, entries, and anchors
