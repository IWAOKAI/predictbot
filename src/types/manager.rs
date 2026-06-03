use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerSummary {
    pub manager_id: String,
    pub owner: String,
    pub balances: Vec<ManagerBalance>,
    pub trading_balance: i64,
    pub open_exposure: i64,
    pub redeemable_value: i64,
    pub realized_pnl: i64,
    pub unrealized_pnl: i64,
    pub account_value: i64,
    pub open_positions: i64,
    pub awaiting_settlement_positions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerBalance {
    pub quote_asset: String,
    pub balance: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerPnl {
    pub manager_id: String,
    pub range: String,
    pub series_type: String,
    pub points: Vec<PnlPoint>,
    pub current_unrealized_pnl: i64,
    pub current_total_pnl: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnlPoint {
    pub timestamp_ms: i64,
    pub realized_pnl: i64,
    pub cumulative_realized_pnl: i64,
}

impl ManagerSummary {
    pub fn realized_pnl_usd(&self) -> f64 {
        self.realized_pnl as f64 / 1e6
    }

    pub fn account_value_usd(&self) -> f64 {
        self.account_value as f64 / 1e6
    }
}
