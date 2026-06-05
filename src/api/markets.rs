use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::client::PredictServerClient;
use crate::engine::{
    compute_edge_score, compute_strike_grid, compute_edge_grid,
    EdgeScore, StrikeGrid, EdgeGrid, MarketAskIndex,
};
use crate::types::{Oracle, OracleState, ManagerSummary};

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
    pub tick_size_usd: f64,
    pub settlement_price_usd: Option<f64>,
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
            tick_size_usd: o.tick_size_usd(),
            settlement_price_usd: o.settlement_price_usd(),
            created_checkpoint: o.created_checkpoint,
        }
    }
}

#[derive(Serialize)]
pub struct MarketDetailResponse {
    pub oracle: MarketSummary,
    pub spot_usd: Option<f64>,
    pub forward_usd: Option<f64>,
    pub seconds_until_expiry: i64,
    pub edge_score: Option<EdgeScore>,
}

#[derive(Serialize)]
pub struct StrikesResponse {
    pub oracle: MarketSummary,
    pub grid: Option<StrikeGrid>,
}

#[derive(Deserialize)]
pub struct StrikesQuery {
    /// 表示するストライク数（デフォルト 11、奇数推奨）
    pub num: Option<usize>,
    /// ストライク間隔（tick の倍数。デフォルト 50 = $50 刻み）
    pub step: Option<i64>,
}

pub async fn list_markets(
    State(state): State<AppState>,
) -> Result<Json<MarketListResponse>, (StatusCode, String)> {
    let oracles = state
        .predict_client
        .list_oracles(PREDICT_ID)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let active_count = oracles.iter().filter(|o| o.is_active()).count();
    let count = oracles.len();

    let mut markets: Vec<MarketSummary> = oracles.iter().map(MarketSummary::from).collect();
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

pub async fn get_market(
    State(state): State<AppState>,
    Path(oracle_id): Path<String>,
) -> Result<Json<MarketDetailResponse>, (StatusCode, String)> {
    let oracle_state: OracleState = state
        .predict_client
        .oracle_state(&oracle_id)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let now_ms = chrono::Utc::now().timestamp_millis();

    let edge_score = match (
        oracle_state.oracle.is_active(),
        &oracle_state.latest_price,
        &oracle_state.latest_svi,
    ) {
        (true, Some(price), Some(svi)) => {
            Some(compute_edge_score(&oracle_state.oracle, price, svi, now_ms))
        }
        _ => None,
    };

    Ok(Json(MarketDetailResponse {
        oracle: MarketSummary::from(&oracle_state.oracle),
        spot_usd: oracle_state.latest_price.as_ref().map(|p| p.spot_usd()),
        forward_usd: oracle_state.latest_price.as_ref().map(|p| p.forward_usd()),
        seconds_until_expiry: oracle_state.oracle.seconds_until_expiry(now_ms),
        edge_score,
    }))
}

pub async fn get_strikes(
    State(state): State<AppState>,
    Path(oracle_id): Path<String>,
    Query(q): Query<StrikesQuery>,
) -> Result<Json<StrikesResponse>, (StatusCode, String)> {
    let oracle_state: OracleState = state
        .predict_client
        .oracle_state(&oracle_id)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let num_strikes = q.num.unwrap_or(11);
    let step_ticks = q.step.unwrap_or(50);
    let now_ms = chrono::Utc::now().timestamp_millis();

    let grid = match (&oracle_state.latest_price, &oracle_state.latest_svi) {
        (Some(price), Some(svi)) => Some(compute_strike_grid(
            &oracle_state.oracle,
            price,
            svi,
            now_ms,
            num_strikes,
            step_ticks,
        )),
        _ => None,
    };

    Ok(Json(StrikesResponse {
        oracle: MarketSummary::from(&oracle_state.oracle),
        grid,
    }))
}


#[derive(Serialize)]
pub struct EdgesResponse {
    pub oracle: MarketSummary,
    pub edge_grid: Option<EdgeGrid>,
}

pub async fn get_edges(
    State(state): State<AppState>,
    Path(oracle_id): Path<String>,
    Query(q): Query<StrikesQuery>,
) -> Result<Json<EdgesResponse>, (StatusCode, String)> {
    let oracle_state = state
        .predict_client
        .oracle_state(&oracle_id)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    // 最新 100 件の mint を取得し、この oracle の market ask index を構築
    let mints = state
        .predict_client
        .positions_minted(100)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    let ask_index = MarketAskIndex::from_mints(&mints, &oracle_id);

    let num_strikes = q.num.unwrap_or(11);
    let step_ticks = q.step.unwrap_or(50);
    let now_ms = chrono::Utc::now().timestamp_millis();

    let edge_grid = match (&oracle_state.latest_price, &oracle_state.latest_svi) {
        (Some(price), Some(svi)) => Some(compute_edge_grid(
            &oracle_state.oracle,
            price,
            svi,
            &ask_index,
            now_ms,
            num_strikes,
            step_ticks,
        )),
        _ => None,
    };

    Ok(Json(EdgesResponse {
        oracle: MarketSummary::from(&oracle_state.oracle),
        edge_grid,
    }))
}


#[derive(Deserialize)]
pub struct ManagerQuery {
    pub owner: String,
}

pub async fn get_manager(
    State(state): State<AppState>,
    Query(q): Query<ManagerQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = state
        .predict_client
        .managers_by_owner(&q.owner)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(Json(result))
}


#[derive(Deserialize)]
pub struct ManagerIdQuery {
    pub manager: String,
}

pub async fn get_positions(
    State(state): State<AppState>,
    Query(q): Query<ManagerIdQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let result = state
        .predict_client
        .manager_positions(&q.manager)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(Json(result))
}

pub async fn get_summary(
    State(state): State<AppState>,
    Query(q): Query<ManagerIdQuery>,
) -> Result<Json<ManagerSummary>, (StatusCode, String)> {
    let result = state
        .predict_client
        .manager_summary(&q.manager)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(Json(result))
}

