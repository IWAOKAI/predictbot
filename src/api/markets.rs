use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::client::PredictServerClient;
use crate::types::Oracle;

const PREDICT_ID: &str = "0xc8736204d12f0a7277c86388a68bf8a194b0a14c5538ad13f22cbd8e2a38028a";

#[derive(Clone)]
pub struct AppState {
    pub predict_client: Arc<PredictServerClient>,
}

#[derive(Serialize)]
pub struct MarketListResponse {
    pub count: usize,
    pub active_count: usize,
    pub markets: Vec<MarketSummary>,
}

#[derive(Serialize)]
pub struct MarketSummary {
    pub oracle_id: String,
    pub underlying_asset: String,
    pub expiry: i64,
    pub expiry_iso: String,
    pub status: String,
    pub min_strike_usd: f64,
    pub created_checkpoint: i64,
}

impl From<&Oracle> for MarketSummary {
    fn from(o: &Oracle) -> Self {
        let expiry_iso = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(o.expiry)
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();

        Self {
            oracle_id: o.oracle_id.clone(),
            underlying_asset: o.underlying_asset.clone(),
            expiry: o.expiry,
            expiry_iso,
            status: format!("{:?}", o.status).to_lowercase(),
            min_strike_usd: o.min_strike_usd(),
            created_checkpoint: o.created_checkpoint,
        }
    }
}

pub async fn list_markets(
    State(state): State<AppState>,
) -> Result<Json<MarketListResponse>, (axum::http::StatusCode, String)> {
    let oracles = state
        .predict_client
        .list_oracles(PREDICT_ID)
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;

    let active_count = oracles.iter().filter(|o| o.is_active()).count();
    let count = oracles.len();

    let mut markets: Vec<MarketSummary> = oracles.iter().map(MarketSummary::from).collect();
    // active を先頭、created_checkpoint 降順（新しい順）
    markets.sort_by(|a, b| {
        let a_active = a.status == "active";
        let b_active = b.status == "active";
        b_active
            .cmp(&a_active)
            .then(b.created_checkpoint.cmp(&a.created_checkpoint))
    });

    Ok(Json(MarketListResponse {
        count,
        active_count,
        markets,
    }))
}
