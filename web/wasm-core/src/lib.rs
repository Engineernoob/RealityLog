use reality_core::{verify, VerifyRequest};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn verify_inclusion(req_json: &str) -> bool {
    match serde_json::from_str::<VerifyRequest>(req_json) {
        Ok(request) => verify(&request).valid,
        Err(_) => false,
    }
}
