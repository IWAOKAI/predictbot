use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSummary {
    pub predict_id: String,
    pub quote_assets: Vec<String>,
    pub vault_balance: i64,
    pub vault_value: i64,
    pub total_mtm: i64,
    pub total_max_payout: i64,
    pub available_liquidity: i64,
    pub available_withdrawal: i64,
    pub plp_total_supply: i64,
    pub plp_share_price: f64,
    pub utilization: f64,
    pub max_payout_utilization: f64,
    pub net_deposits: i64,
    pub total_supplied: i64,
    pub total_withdrawn: i64,
}

impl VaultSummary {
    pub fn vault_value_usd(&self) -> f64 {
        self.vault_value as f64 / 1e6
    }

    pub fn available_liquidity_usd(&self) -> f64 {
        self.available_liquidity as f64 / 1e6
    }

    pub fn utilization_pct(&self) -> f64 {
        self.utilization * 100.0
    }
}
