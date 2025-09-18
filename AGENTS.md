# Repository Guidelines

## Project Structure & Module Organization
RealityLog is a Rust workspace. `crates/core` holds Merkle primitives and shared API types, `crates/logd` is the Axum daemon, and `crates/anchor` is the root anchorer loop. Web assets live under `web`: `wasm-core` exposes proof verification via wasm-bindgen, while `verifier-ext` is the Vite + TypeScript UI. All file-backed state (`leaves.json`, `entries.json`, `anchors.json`) sits in the top-level `data/` directory. Tests for Rust crates belong in their respective `src/` modules or `tests/` directories alongside the code they exercise.

## Build, Test, and Development Commands
Bootstrap once per clone: `rustup target add wasm32-unknown-unknown` and install `wasm-pack`. Run the log daemon with `cargo run -p reality-logd`. Execute `cargo test -p reality-core` before shipping proofs logic. Build the WASM bundle using `wasm-pack build web/wasm-core --target web --out-dir pkg`. Launch the verifier UI with `cd web/verifier-ext && npm install && npm run dev`. When touching multiple crates, prefer `cargo fmt` and `cargo clippy --workspace` to keep style consistent.

## Coding Style & Naming Conventions
Adhere to Rust 2021 idioms: 4-space indentation, snake_case for functions, and UpperCamelCase for types. Exposed JSON structs derive `Serialize`/`Deserialize` with lowercase field names. Keep module names short and descriptive (`storage.rs`, `routes.rs`) and stay consistent with crate naming (`reality-core`, `reality-logd`, `reality-anchor`). For TypeScript, use ESLint-compatible formatting (2 spaces, trailing commas) and favour explicit types on exported members. WASM exports must remain snake_case to match the Rust functions.

## Testing Guidelines
Unit tests in `reality-core` confirm hashing determinism, deterministic roots, and proof round-trips—mirror those patterns when extending functionality. Add integration tests under `crates/logd` if routes gain business logic (use `axum::Router` with `tower::ServiceExt`). For the web verifier, add lightweight Playwright or Vitest checks once the UI becomes interactive beyond a single button.

## Commit & Pull Request Guidelines
Write commits in imperative mood with Conventional Commit prefixes when scope is clear (`feat(logd): add pagination for anchors`). Include reproduction steps, proof JSON, or curl snippets in PR descriptions so reviewers can replay the flow. Reference issue IDs where applicable and update the README if the operational story changes. Before requesting review, capture test runs (`cargo test`, `wasm-pack build`, `npm run build`) in the PR notes.
