use serde::{Deserialize, Serialize};

/// DeepBook Predict Oracle のメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oracle {
    pub predict_id: String,
    pub oracle_id: String,
    pub oracle_cap_id: String,
    pub underlying_asset: String,
    pub expiry: i64,
    pub min_strike: i64,
    pub tick_size: i64,
    pub status: OracleStatus,
    pub activated_at: i64,
    pub settlement_price: Option<i64>,
    pub settled_at: Option<i64>,
    pub created_checkpoint: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OracleStatus {
    Inactive,
    Active,
    PendingSettlement,
    Settled,
}

/// Oracle の最新価格（spot + forward）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OraclePrice {
    pub oracle_id: String,
    pub checkpoint: i64,
    pub checkpoint_timestamp_ms: i64,
    pub spot: i64,
    pub forward: i64,
    pub onchain_timestamp: i64,
}

/// SVI パラメータ（ボラティリティスマイル）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleSvi {
    pub oracle_id: String,
    pub checkpoint: i64,
    pub checkpoint_timestamp_ms: i64,
    pub a: i64,
    pub b: i64,
    pub rho: i64,
    pub rho_negative: bool,
    pub m: i64,
    pub m_negative: bool,
    pub sigma: i64,
    pub onchain_timestamp: i64,
}

/// Oracle 状態（Oracle + 最新 price + 最新 SVI）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleState {
    pub oracle: Oracle,
    pub latest_price: Option<OraclePrice>,
    pub latest_svi: Option<OracleSvi>,
    pub ask_bounds: Option<serde_json::Value>,
}

impl Oracle {
    /// 価格を人間が読める形に（8桁精度から USD に変換）
    pub fn min_strike_usd(&self) -> f64 {
        self.min_strike as f64 / 1e8
    }

    /// expiry が現在から何秒後か
    pub fn seconds_until_expiry(&self, now_ms: i64) -> i64 {
        (self.expiry - now_ms) / 1000
    }

    /// アクティブかどうか
    pub fn is_active(&self) -> bool {
        self.status == OracleStatus::Active
    }
}

impl OraclePrice {
    /// spot を USD に
    pub fn spot_usd(&self) -> f64 {
        self.spot as f64 / 1e8
    }

    /// forward を USD に
    pub fn forward_usd(&self) -> f64 {
        self.forward as f64 / 1e8
    }

    /// フォワード偏差（市場の方向感）
    pub fn forward_bias(&self) -> f64 {
        (self.forward as f64 - self.spot as f64) / self.spot as f64
    }
}
