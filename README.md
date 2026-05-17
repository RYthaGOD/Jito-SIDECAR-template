<p align="center">
  <h1>⚡ Jito FCFS Sidecar Template</h1>
  <p align="center">
    <strong>A high-fidelity framework for building strict First-Come-First-Served (FCFS) transaction relayers on Solana using Jito Bundles.</strong>
  </p>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.75+-orange?logo=rust&style=flat-square" />
  <img src="https://img.shields.io/badge/Solana-2.2.1-blue?logo=solana&style=flat-square" />
  <img src="https://img.shields.io/badge/Jito-MEV-brightgreen?style=flat-square" />
</p>

---

> [!TIP]
> **🤖 FOR AI AGENTS**: If you are an AI coding assistant, please start by reading the **[Agent Intelligence Layer](.agents/AGENT.md)** to understand the architectural patterns and extension hooks of this repository.

---

## ❓ What is this?

Normally on Solana, if two people want to do the exact same thing at the exact same time (like buy a limited NFT), the person who pays the highest **Priority Fee** wins. This creates toxic MEV (Maximal Extractable Value) gas wars and ruins the experience for regular users.

This **Jito FCFS Sidecar** completely bypasses the mempool to fix this:
1. Users send their requests directly to your off-chain Sidecar (this template) instead of the Solana RPC.
2. The Sidecar stamps the exact millisecond the request arrived.
3. It bundles all requests together in strict order and sends them directly to the **Jito Block Engine**.
4. The Jito Block Engine guarantees the transactions are executed sequentially. **Priority fees no longer matter. The fastest clicker always wins.**

---

## 🧠 The Agnostic Plugin Architecture

This sidecar doesn't care if you are building a DePIN network, a Web3 Game, an NFT Mint, or a Private Liquidity Pool. It only handles the networking and Jito bundling. 

To use it for your specific application, you just implement the `BamPlugin` trait.

---

## 🚀 How To Use It (Example: NFT Minting)

Let's say you want to use this sidecar to protect your NFT Mint from front-running bots.

### Step 1: Define your custom Payload
Create a new file (e.g., `src/nft_mint_impl.rs`) and define the data you expect users to send to your sidecar.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NftMintPayload {
    pub buyer_wallet: String,
    pub collection_id: String,
    pub quantity: u8,
}
```

### Step 2: Implement the `BamPlugin` Trait
Tell the sidecar how to verify the request and turn it into Solana instructions.

```rust
use crate::plugin::BamPlugin;
use async_trait::async_trait;
use solana_sdk::{instruction::{AccountMeta, Instruction}, pubkey::Pubkey};
use anyhow::Result;

pub struct NftMintPlugin;

#[async_trait]
impl BamPlugin for NftMintPlugin {
    type Payload = NftMintPayload;

    fn id(&self) -> &'static str {
        "nft-mint-plugin"
    }

    // (Optional) Verify signatures, off-chain auth, or rate limits here
    async fn verify(&self, payload: &Self::Payload) -> Result<()> {
        if payload.quantity > 5 {
            return Err(anyhow::anyhow!("Cannot mint more than 5"));
        }
        Ok(())
    }

    // Translate the payload into Solana instructions
    async fn build_instructions(
        &self,
        payload: &Self::Payload,
        authority: &Pubkey,
    ) -> Result<Vec<Instruction>> {
        
        // Example: Creating the instruction for your custom Smart Contract
        let program_id = "YourProgramId1111111111111111111111111111111".parse().unwrap();
        
        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(*authority, true), // The sidecar paying the fees
                AccountMeta::new(payload.buyer_wallet.parse()?, false), // The buyer receiving the NFT
            ],
            data: bincode::serialize(&payload.quantity)?, // Tell the contract how many to mint
        };

        Ok(vec![ix])
    }
}
```

### Step 3: Plug it into the Main Engine
Open `src/main.rs` and swap out the default plugin with your new one:

```rust
// In src/main.rs
use jito_bam_template::nft_mint_impl::NftMintPlugin;

// Replace the old plugin initialization
let plugin = Arc::new(NftMintPlugin);
```

### Step 4: Run your Private Sequencer!
```bash
cargo run --release
```

Now, your users can POST their JSON payloads directly to `http://localhost:3030/submit`. The sidecar will automatically batch them, enforce strict FCFS ordering, and execute them perfectly on-chain via Jito.

---

## 🛠️ Components Overview

- **`src/main.rs`**: The core async engine. Handles HTTP requests, millisecond FCFS timestamping, and payload batching.
- **`src/plugin.rs`**: The `BamPlugin` trait definition.
- **`src/bundler.rs`**: High-performance integration with Jito's JSON-RPC SDK for packing sequential transaction bundles.
- **`src/zk.rs`**: Pre-integrated wrapper for ZK-Compression (Light Protocol) state proofs.

---

## 🤝 Contributing

Contributions are welcome! If you have suggestions for improving the FCFS aggregation logic, feel free to open a PR.

## ⚖️ License

Distributed under the MIT License. See `LICENSE` for more information.
