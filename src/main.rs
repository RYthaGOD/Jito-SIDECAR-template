use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use jito_bam_template::{
    example_impl::{ExampleHeartbeatPlugin, HeartbeatPayload},
    plugin::BamPlugin,
    bundler::JitoBundler,
    zk::ZkModule,
};
use solana_sdk::signature::{Keypair, Signer};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error, warn};
use std::time::Instant;
use solana_sdk::instruction::Instruction;

pub struct OrderedPayload<T> {
    pub arrival_time: Instant,
    pub payload: T,
}

struct AppState<P: BamPlugin> {
    pub plugin: Arc<P>,
    pub bundler: Arc<JitoBundler>,
    pub zk_module: Option<Arc<tokio::sync::Mutex<ZkModule>>>,
    pub tx_queue: mpsc::Sender<OrderedPayload<P::Payload>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize professional tracing subscriber for production monitoring
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    info!("🛠️ Starting Jito BAM Sidecar initialization...");

    // 1. Load System Configuration
    // Authority key is used both as the fee payer and the batch signer
    let authority = Keypair::from_base58_string(
        &std::env::var("BAM_AUTHORITY_KEY").expect("CRITICAL: BAM_AUTHORITY_KEY must be set")
    );
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let jito_url = std::env::var("JITO_URL").unwrap_or_else(|_| "https://mainnet.block-engine.jito.wtf/api/v1/bundles".to_string());
    let photon_url = std::env::var("PHOTON_URL").ok();

    // 2. Initialize Framework Components
    // ExampleHeartbeatPlugin is a demonstration. Swap this with your specific BAM implementation.
    let plugin = Arc::new(ExampleHeartbeatPlugin);
    let bundler = Arc::new(JitoBundler::new(&jito_url, &rpc_url, authority));
    
    // Conditional ZK-Module initialization based on environment configuration
    let zk_module = if let Some(p_url) = photon_url {
        Some(Arc::new(tokio::sync::Mutex::new(ZkModule::new(&rpc_url, &p_url).await?)))
    } else {
        warn!("⚠️  PHOTON_URL not provided. Building transactions without ZK-Compression proofs.");
        None
    };

    let (tx_queue, rx_queue) = mpsc::channel(1024);

    let state = Arc::new(AppState {
        plugin,
        bundler,
        zk_module,
        tx_queue,
    });

    // 3. Spawn Aggregator Loop
    let state_clone = state.clone();
    tokio::spawn(async move {
        aggregator_loop(state_clone, rx_queue).await;
    });

    // 4. Start HTTP API
    let app = Router::new()
        .route("/submit", post(submit_handler::<ExampleHeartbeatPlugin>))
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3030));
    info!("🚀 Jito BAM Plugin Template active on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn submit_handler<P: BamPlugin>(
    State(state): State<Arc<AppState<P>>>,
    Json(payload): Json<P::Payload>,
) -> Result<&'static str, (StatusCode, String)> {
    
    // 1. Framework-level Verification
    if let Err(e) = state.plugin.verify(&payload).await {
        return Err((StatusCode::UNAUTHORIZED, format!("Verification failed: {}", e)));
    }

    // 2. Wrap and Queue for Aggregation
    let ordered_payload = OrderedPayload {
        arrival_time: Instant::now(),
        payload,
    };

    if let Err(_) = state.tx_queue.try_send(ordered_payload) {
        return Err((StatusCode::TOO_MANY_REQUESTS, "Queue full".to_string()));
    }

    Ok("Accepted")
}

async fn aggregator_loop<P: BamPlugin>(
    state: Arc<AppState<P>>,
    mut rx: mpsc::Receiver<OrderedPayload<P::Payload>>,
) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    let mut batch = Vec::new();

    loop {
        tokio::select! {
            Some(payload) = rx.recv() => {
                batch.push(payload);
                if batch.len() >= state.plugin.batch_size() {
                    process_batch(&state, batch.drain(..).collect()).await;
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() {
                    process_batch(&state, batch.drain(..).collect()).await;
                }
            }
        }
    }
}

async fn process_batch<P: BamPlugin>(
    state: &AppState<P>,
    mut batch: Vec<OrderedPayload<P::Payload>>,
) {
    info!("📦 Processing batch of {} items", batch.len());
    
    // Ensure strict FCFS ordering by sorting based on arrival time
    batch.sort_by_key(|p| p.arrival_time);

    // Each payload will have its instructions placed in a separate chunk
    // to become sequential transactions in the Jito bundle.
    let mut instruction_chunks: Vec<Vec<Instruction>> = Vec::new();
    let authority_pubkey = state.bundler.authority.pubkey();

    for ordered in batch {
        let payload = ordered.payload;
        // Optional: ZK-State resolution before building instructions
        if let Some(zk) = &state.zk_module {
            // let mut zk_locked = zk.lock().await;
            // zk_locked.get_compressed_account(...).await;
        }

        match state.plugin.build_instructions(&payload, &authority_pubkey).await {
            Ok(ixs) => instruction_chunks.push(ixs),
            Err(e) => error!("❌ Failed to build instructions for payload: {}", e),
        }
    }

    if !instruction_chunks.is_empty() {
        match state.bundler.send_bundle_with_tip(instruction_chunks, state.plugin.get_tip_amount()).await {
            Ok(id) => {
                let bundler = state.bundler.clone();
                tokio::spawn(async move {
                    bundler.watch_bundle(id).await;
                });
            }
            Err(e) => error!("❌ Bundle submission failed: {}", e),
        }
    }
}
