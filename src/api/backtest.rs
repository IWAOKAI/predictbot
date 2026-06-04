use axum::{extract::State, http::StatusCode, Json};

use crate::api::markets::AppState;
use crate::engine::{run_calibration, CalibrationReport};

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
