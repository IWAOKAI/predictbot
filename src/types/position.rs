use serde::{Deserialize, Serialize};

/// positions/minted の1件（binary position の mint イベント）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionMint {
    pub oracle_id: String,
    pub predict_id: String,
    pub manager_id: String,
    pub trader: String,
    pub expiry: i64,
    pub strike: i64,
    pub is_up: bool,
    pub quantity: i64,
    pub cost: i64,
    pub ask_price: i64,
    pub checkpoint_timestamp_ms: i64,
}

impl PositionMint {
    /// ask_price を確率（0.0〜1.0+）に。1e9 スケール。
    pub fn ask_probability(&self) -> f64 {
        self.ask_price as f64 / 1e9
    }

    /// strike を USD に（1e9 スケール）
    pub fn strike_usd(&self) -> f64 {
        self.strike as f64 / 1e9
    }
}
