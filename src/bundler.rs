use jito_sdk_rust::jito_json_rpc_sdk::JitoJsonRpcSDK;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use tracing::{info, error, warn};
use anyhow::{Result, anyhow};
use std::sync::Arc;

/// High-performance interface for Jito Bundle management.
/// Responsible for transaction construction, tip account resolution, and Block Engine integration.
pub struct JitoBundler {
    pub jito_sdk: JitoJsonRpcSDK,
    pub rpc_client: Arc<RpcClient>,
    pub authority: Keypair,
}

impl JitoBundler {
    pub fn new(jito_url: &str, rpc_url: &str, authority: Keypair) -> Self {
        Self {
            jito_sdk: JitoJsonRpcSDK::new(jito_url, None),
            rpc_client: Arc::new(RpcClient::new(rpc_url)),
            authority,
        }
    }

    pub async fn send_bundle_with_tip(
        &self,
        instructions: Vec<Instruction>,
        tip_lamports: u64,
    ) -> Result<String> {
        let blockhash = self.rpc_client.get_latest_blockhash()?;
        
        // 1. Fetch Jito Tip Account
        let tip_accounts = self.jito_sdk.get_tip_accounts().await
            .map_err(|e| anyhow!("Failed to fetch tip accounts: {}", e))?;
        
        let tip_pubkey: Pubkey = tip_accounts[0].parse()
            .map_err(|_| anyhow!("Invalid tip pubkey"))?;

        // 2. Prepare Transactions
        let mut txs = Vec::new();

        // Main transaction with instructions
        let main_tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.authority.pubkey()),
            &[&self.authority],
            blockhash,
        );
        txs.push(main_tx);

        // Tip transaction
        let tip_tx = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &self.authority.pubkey(),
                &tip_pubkey,
                tip_lamports,
            )],
            Some(&self.authority.pubkey()),
            &[&self.authority],
            blockhash,
        );
        txs.push(tip_tx);

        // 3. Serialize and Send
        let bundle: Vec<String> = txs
            .iter()
            .map(|tx| bs58::encode(bincode::serialize(tx).unwrap()).into_string())
            .collect();

        let bundle_id = self.jito_sdk.send_bundle(bundle).await
            .map_err(|e| anyhow!("Bundle rejected: {}", e))?;

        info!("🚀 Bundle {} submitted to Jito.", bundle_id);
        Ok(bundle_id)
    }

    pub async fn watch_bundle(&self, bundle_id: String) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        for _ in 0..15 {
            interval.tick().await;
            if let Ok(statuses) = self.jito_sdk.get_bundle_statuses(vec![bundle_id.clone()]).await {
                if let Some(s) = statuses.first() {
                    match s.status.as_str() {
                        "Landed" => {
                            info!("🎉 Bundle {} CONFIRMED!", bundle_id);
                            return;
                        }
                        "Failed" => {
                            error!("❌ Bundle {} FAILED.", bundle_id);
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
        warn!("⌛ Bundle {} status unknown after timeout.", bundle_id);
    }
}
