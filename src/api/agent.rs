use axum::{extract::State, http::StatusCode, Json};
use serde_json::Value;
use std::process::Command;

use crate::api::markets::AppState;

/// POST /api/agent/run
/// Runs ONE full two-agent autonomous cycle (Strategist proposes, Risk
/// Officer reviews/vetoes), stores the decision on Walrus, hashes it,
/// enforces the Mandate on-chain, and verifies the blob -- returning the
/// structured per-stage result as JSON.
pub async fn run_agent(
    State(_state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // The cycle calls the Claude API + Walrus + an on-chain PTB, so it can
    // take ~20-40s. We run the existing, proven Python loop in --json mode.
    let output = tokio::task::spawn_blocking(|| {
        Command::new("python3")
            .arg("scripts/deepedge_loop_api.py")
            .arg("--json")
            .current_dir("/root/deepedge")
            .output()
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("join error: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("spawn error: {e}")))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err((StatusCode::BAD_GATEWAY, format!("loop failed: {err}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The script prints a single JSON object on stdout.
    let parsed: Value = serde_json::from_str(stdout.trim())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR,
            format!("bad JSON from loop: {e}; raw: {stdout}")))?;
    Ok(Json(parsed))
}

/// GET /api/agent/status
/// Returns the current Mandate state (per-bet cap, budget, spent, active)
/// by querying the on-chain object via the Sui fullnode RPC.
pub async fn agent_status(
    State(_state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    const MANDATE: &str = "0x753fb2e637d42067aeea59df6044ddfeb37ac22c92f28c89a8ffc6e3a4635f3a";
    const RPC: &str = "https://fullnode.testnet.sui.io";

    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "sui_getObject",
        "params": [MANDATE, {"showContent": true}]
    });

    let client = reqwest::Client::new();
    let resp = client.post(RPC).json(&body).send().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    let json: Value = resp.json().await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    // Extract the fields we care about; fall back to the raw object.
    let fields = &json["result"]["data"]["content"]["fields"];
    let out = serde_json::json!({
        "mandate_id": MANDATE,
        "per_bet_cap": fields["per_bet_cap"],
        "total_budget": fields["total_budget"],
        "spent": fields["spent"],
        "active": fields["active"],
    });
    Ok(Json(out))
}
