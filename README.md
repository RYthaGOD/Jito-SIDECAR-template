<p align="center">
  <h1>⚡ Jito BAM Maker Quote Updater (Sidecar)</h1>
  <p align="center">
    <strong>A high-frequency, deduplicating sidecar for Market Makers on Solana using Jito Block Assembly Marketplace (BAM).</strong>
  </p>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.75+-orange?logo=rust&style=flat-square" />
  <img src="https://img.shields.io/badge/Solana-2.2.1-blue?logo=solana&style=flat-square" />
  <img src="https://img.shields.io/badge/Jito-MEV-brightgreen?style=flat-square" />
  <img src="https://img.shields.io/badge/High%20Frequency-50ms-red?style=flat-square" />
</p>

---

## ❓ What is this?

Market Makers on Solana often struggle with **network jitter and scheduler randomness**. When quoting on proprietary AMMs or orderbooks, being unable to guarantee exactly *when* a transaction lands leads to:
- **Stale Quotes**: Quotes getting picked off by predatory MEV searchers.
- **Transaction Spam**: Forced to spam hundreds of updates a second hoping one lands.
- **Wider Spreads**: Because of execution uncertainty, spreads must be widened to account for toxic flow.

This **Jito Maker Quote Updater Sidecar** solves this by bypassing the standard mempool:
1. Your trading bot sends quote updates to this sidecar instead of an RPC.
2. The sidecar **deduplicates** the quotes in real-time, keeping only the freshest quote per market.
3. Every **50ms**, it emits the deduplicated quotes directly to the Jito Block Engine.
4. Jito BAM guarantees predictable, top-of-batch execution for your quotes.

---

## 🚀 Key Features

- **Real-Time Deduplication**: If your bot sends 20 quotes for `SOL/USDC` within 40ms, the sidecar discards the older 19 quotes, saving massive amounts of SOL in transaction fees.
- **Deterministic 50ms Cadence**: Emits Jito bundles exactly every 50ms, giving you a massive edge in predictable execution.
- **Ed25519 Cryptographic Security**: Validates signatures on incoming payloads so unauthorized actors cannot spoof your quotes.
- **DDoS Protection**: Enforces a strict `ALLOWED_MARKETS` configuration so malicious users cannot exhaust the sidecar's memory.

---

## ⚙️ Setup & Configuration

### 1. Environment Variables
Copy `.env.example` to `.env` and configure your settings:

```env
# The Base58 encoded public key of your market making bot
MAKER_PUBKEY=YOUR_BOT_PUBKEY

# Comma-separated list of allowed market IDs to prevent DDoS
ALLOWED_MARKETS=SOL/USDC,BTC/USDC

# The interval (in milliseconds) at which the aggregator emits Jito bundles
BAM_TICK_RATE_MS=50

# Authority key paying for the bundles
BAM_AUTHORITY_KEY=YOUR_AUTHORITY_PRIVATE_KEY
```

### 2. Customizing the Instructions
Open `src/maker_plugin.rs` and update the `build_instructions` method to point to your specific AMM or Orderbook Program ID, and serialize your payload into the format your program expects.

---

## 🛠️ Testing & Usage

### 1. Start the Sidecar
```bash
cargo run --release
```

### 2. Generate a Test Payload
We provide a utility script to demonstrate how your Market Making bot should sign the quote update bytes using a private key and serialize it over HTTP.

Run the payload generator:
```bash
cargo run --bin generate_payload
```

This will print out a dummy `MAKER_PUBKEY` (add it to your `.env`) and output a `curl` command with a cryptographically valid payload that you can paste into your terminal to test the `/submit` endpoint.

---

## 🏗️ Architecture

- **`src/main.rs`**: The core async engine. Handles the `HashMap` deduplication logic and the strict 50ms `tokio::time::interval` loop.
- **`src/maker_plugin.rs`**: The implementation of the `BamPlugin` trait customized for Market Makers (payload structure, signature verification, and allow-listing).
- **`src/bundler.rs`**: High-performance integration with Jito's JSON-RPC SDK for packing sequential transaction bundles.

---

## ⚖️ License
Distributed under the MIT License. See `LICENSE` for more information.
