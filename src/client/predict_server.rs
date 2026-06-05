use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

use crate::types::{Oracle, OracleState, ManagerSummary, ManagerPnl, VaultSummary, PositionMint};

const DEFAULT_BASE_URL: &str = "https://predict-server.testnet.mystenlabs.com";

/// DeepBook Predict Public Server のクライアント
#[derive(Clone)]
pub struct PredictServerClient {
    base_url: String,
    http: Client,
}

impl PredictServerClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            http,
        })
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            base_url: base_url.into(),
            http,
        })
    }

    /// GET /status
    pub async fn status(&self) -> Result<serde_json::Value> {
        let url = format!("{}/status", self.base_url);
        let res = self.http.get(&url).send().await?.error_for_status()?;
        Ok(res.json().await?)
    }

    /// GET /predicts/:predict_id/oracles
    pub async fn list_oracles(&self, predict_id: &str) -> Result<Vec<Oracle>> {
        let url = format!("{}/predicts/{}/oracles", self.base_url, predict_id);
        let res = self.http.get(&url).send().await?.error_for_status()?;
        Ok(res.json().await?)
    }

    /// 同上、status == "active" のみフィルター
    pub async fn list_active_oracles(&self, predict_id: &str) -> Result<Vec<Oracle>> {
        let all = self.list_oracles(predict_id).await?;
        Ok(all.into_iter().filter(|o| o.is_active()).collect())
    }

    /// GET /oracles/:oracle_id/state
    pub async fn oracle_state(&self, oracle_id: &str) -> Result<OracleState> {
        let url = format!("{}/oracles/{}/state", self.base_url, oracle_id);
        let res = self.http.get(&url).send().await?.error_for_status()?;
        Ok(res.json().await?)
    }

    /// GET /predicts/:predict_id/vault/summary
    pub async fn vault_summary(&self, predict_id: &str) -> Result<VaultSummary> {
        let url = format!("{}/predicts/{}/vault/summary", self.base_url, predict_id);
        let res = self.http.get(&url).send().await?.error_for_status()?;
        Ok(res.json().await?)
    }

    /// GET /managers/:manager_id/summary
    pub async fn manager_summary(&self, manager_id: &str) -> Result<ManagerSummary> {
        let url = format!("{}/managers/{}/summary", self.base_url, manager_id);
        let res = self.http.get(&url).send().await?.error_for_status()?;
        Ok(res.json().await?)
    }

    /// GET /managers/:manager_id/pnl?range=ALL
    pub async fn manager_pnl(&self, manager_id: &str, range: &str) -> Result<ManagerPnl> {
        let url = format!("{}/managers/{}/pnl?range={}", self.base_url, manager_id, range);
        let res = self.http.get(&url).send().await?.error_for_status()?;
        Ok(res.json().await?)
    }

    /// GET /positions/minted?limit=N
    /// 最新 N 件の binary position mint を取得（全 oracle 横断、サーバー側 oracle フィルタは無い）
    /// GET /managers?owner=ADDR
    /// 指定 owner の PredictManager イベントを返す（無ければ空配列）
    pub async fn managers_by_owner(&self, owner: &str) -> Result<serde_json::Value> {
        let url = format!("{}/managers?owner={}", self.base_url, owner);
        let res = self.http.get(&url).send().await?.error_for_status()?;
        Ok(res.json().await?)
    }

    pub async fn positions_minted(&self, limit: usize) -> Result<Vec<PositionMint>> {
        let url = format!("{}/positions/minted?limit={}", self.base_url, limit);
        let res = self.http.get(&url).send().await?.error_for_status()?;
        Ok(res.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PREDICT_ID: &str = "0xc8736204d12f0a7277c86388a68bf8a194b0a14c5538ad13f22cbd8e2a38028a";

    #[tokio::test]
    #[ignore = "hits live Predict Server; run with --ignored"]
    async fn test_status() {
        let client = PredictServerClient::new().unwrap();
        let status = client.status().await.unwrap();
        assert_eq!(status["status"], "OK");
    }

    #[tokio::test]
    #[ignore = "hits live Predict Server; run with --ignored"]
    async fn test_list_oracles() {
        let client = PredictServerClient::new().unwrap();
        let oracles = client.list_oracles(PREDICT_ID).await.unwrap();
        assert!(!oracles.is_empty());
    }
}
