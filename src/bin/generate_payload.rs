use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;

fn main() {
    // Standardized payload generator for local simulation and integration testing.
    // Generates a mock Ed25519 node address and dummy data metrics.
    let signing_key = SigningKey::from_bytes(&[1u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let node_address = bs58::encode(verifying_key.to_bytes()).into_string();

    let payload = json!({
        "node_address": node_address,
        "timestamp": chrono::Utc::now().timestamp(),
        "data_root": [42u8; 32],
    });

    println!("🚀 Example Payload for /submit:");
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
}
