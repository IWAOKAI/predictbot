use axum::{extract::State, http::StatusCode, Json};

use crate::api::markets::AppState;
use crate::engine::{run_calibration, CalibrationReport, run_accuracy, AccuracyReport};
use std::collections::{HashMap, HashSet};

const PREDICT_ID: &str = "0xc8736204d12f0a7277c86388a68bf8a194b0a14c5538ad13f22cbd8e2a38028a";

/// GET /api/backtest/calibration
/// 過去の settled oracle に対する全 mint を集計し、市場 ask のキャリブレーションを測る。
pub async fn calibration(
    State(state): State<AppState>,
) -> Result<Json<CalibrationReport>, (StatusCode, String)> {
    // 1. 全 oracle（settlement_price 込み）を取得
    let oracles = state
        .predict_client
        .list_oracles(PREDICT_ID)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    // 2. 直近 1000 件の mint を取得
    let mints = state
        .predict_client
        .positions_minted(2000)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    // 3. キャリブレーション計算
    let report = run_calibration(&oracles, &mints);
    Ok(Json(report))
}


/// GET /api/backtest/accuracy
/// DeepEdge fair vs 市場 ask の予測精度を Brier Score で比較。
/// mint がある settled oracle のみ対象（API コール数を抑制）。
pub async fn accuracy(
    State(state): State<AppState>,
) -> Result<Json<AccuracyReport>, (StatusCode, String)> {
    let client = &state.predict_client;

    // 1. 全 oracle 取得（settlement_price 込み）
    let oracles = client
        .list_oracles(PREDICT_ID)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    let settlement_map: HashMap<String, i64> = oracles
        .iter()
        .filter_map(|o| o.settlement_price.map(|s| (o.oracle_id.clone(), s)))
        .collect();

    // 2. mint 取得
    let mints = client
        .positions_minted(2000)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    // 3. mint がある settled oracle の id を集める
    let target_oracles: HashSet<String> = mints
        .iter()
        .filter(|m| settlement_map.contains_key(&m.oracle_id))
        .map(|m| m.oracle_id.clone())
        .collect();

    // 4. 各 target oracle の state（svi/price）を取得
    let mut states: HashMap<String, (i64, _, _)> = HashMap::new();
    for oid in target_oracles.iter() {
        if let Ok(st) = client.oracle_state(oid).await {
            if let (Some(svi), Some(price)) = (st.latest_svi, st.latest_price) {
                if let Some(&settle) = settlement_map.get(oid) {
                    states.insert(oid.clone(), (settle, svi, price));
                }
            }
        }
    }

    let report = run_accuracy(&states, &mints);
    Ok(Json(report))
}

