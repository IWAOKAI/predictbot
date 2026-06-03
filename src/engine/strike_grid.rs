//! Strike grid: あるオラクルに対して、forward 周辺の複数ストライクで
//! 公正確率（UP / DOWN）を計算する。
//!
//! 表示用の MVP として:
//!   - forward の周りに num_strikes 個のストライクを選ぶ
//!   - DeepBook Predict の tick_size に揃える
//!   - 各ストライクで P(UP), P(DOWN) を計算
//!
//! 時間調整:
//!   SVI が返す total_variance は「満期までの分散」だが、現在から満期までの
//!   残り時間で再スケールしたいケースもある。今は素直に w(k) をそのまま使う
//!   （DeepBook Predict のオラクルが返す SVI が「残り T」基準である前提）。

use serde::Serialize;

use crate::engine::pricing::{binary_call_probability, binary_put_probability};
use crate::engine::svi::SviParams;
use crate::types::{Oracle, OraclePrice, OracleSvi};

#[derive(Debug, Clone, Serialize)]
pub struct StrikeFairProbability {
    pub strike_usd: f64,
    pub log_moneyness: f64,
    pub total_variance: f64,
    pub implied_vol_annualized: Option<f64>,
    pub fair_up: f64,
    pub fair_down: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrikeGrid {
    pub spot_usd: f64,
    pub forward_usd: f64,
    pub seconds_until_expiry: i64,
    pub atm_strike_usd: f64,
    pub strikes: Vec<StrikeFairProbability>,
}

/// 標準的な ATM の選び方: forward を tick_size に丸める
fn round_to_tick(value_usd: f64, tick_usd: f64) -> f64 {
    if tick_usd <= 0.0 {
        return value_usd;
    }
    (value_usd / tick_usd).round() * tick_usd
}

/// num_strikes 個のストライクを atm の上下に等間隔で生成
fn strike_grid_usd(atm_usd: f64, tick_usd: f64, step_ticks: i64, num_strikes: usize) -> Vec<f64> {
    if num_strikes == 0 {
        return vec![];
    }
    let half = (num_strikes / 2) as i64;
    let step_usd = tick_usd * step_ticks as f64;
    (-half..=half)
        .take(num_strikes)
        .map(|i| (atm_usd + step_usd * i as f64).max(0.0))
        .collect()
}

const SECONDS_PER_YEAR: f64 = 365.25 * 24.0 * 3600.0;

/// Oracle 状態から StrikeGrid を計算
pub fn compute_strike_grid(
    oracle: &Oracle,
    price: &OraclePrice,
    svi: &OracleSvi,
    now_ms: i64,
    num_strikes: usize,
    step_ticks: i64,
) -> StrikeGrid {
    let svi_params = SviParams::from_event(svi);
    let forward = price.forward_usd();
    let spot = price.spot_usd();
    let tick = oracle.tick_size_usd();

    let atm = round_to_tick(forward, tick);
    let strikes_usd = strike_grid_usd(atm, tick, step_ticks, num_strikes);

    let seconds_to_expiry = oracle.seconds_until_expiry(now_ms).max(0);
    let time_to_expiry_years = (seconds_to_expiry as f64) / SECONDS_PER_YEAR;

    let strikes: Vec<StrikeFairProbability> = strikes_usd
        .iter()
        .map(|&k_usd| {
            // SVI は log-moneyness ln(K/F) で評価
            let log_moneyness = if forward > 0.0 && k_usd > 0.0 {
                (k_usd / forward).ln()
            } else {
                0.0
            };
            let w = svi_params.total_variance(log_moneyness);
            let iv = if time_to_expiry_years > 0.0 && w > 0.0 {
                Some((w / time_to_expiry_years).sqrt())
            } else {
                None
            };
            let fair_up = binary_call_probability(forward, k_usd, w);
            let fair_down = binary_put_probability(forward, k_usd, w);
            StrikeFairProbability {
                strike_usd: k_usd,
                log_moneyness,
                total_variance: w,
                implied_vol_annualized: iv,
                fair_up,
                fair_down,
            }
        })
        .collect();

    StrikeGrid {
        spot_usd: spot,
        forward_usd: forward,
        seconds_until_expiry: oracle.seconds_until_expiry(now_ms),
        atm_strike_usd: atm,
        strikes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_size_matches_request() {
        let oracle = Oracle {
            predict_id: "".into(),
            oracle_id: "".into(),
            oracle_cap_id: "".into(),
            underlying_asset: "BTC".into(),
            expiry: 2_000_000_000_000,
            min_strike: 50_000_000_000_000,
            tick_size: 1_000_000_000,
            status: crate::types::OracleStatus::Active,
            activated_at: 1_900_000_000_000,
            settlement_price: None,
            settled_at: None,
            created_checkpoint: 0,
        };
        let price = OraclePrice {
            oracle_id: "".into(),
            checkpoint: 0,
            checkpoint_timestamp_ms: 0,
            spot: 66_000_000_000_000,
            forward: 66_000_000_000_000,
            onchain_timestamp: 0,
        };
        let svi = OracleSvi {
            oracle_id: "".into(),
            checkpoint: 0,
            checkpoint_timestamp_ms: 0,
            a: 36_000,
            b: 600_000,
            rho: 940_000_000,
            rho_negative: true,
            m: 19_900_000,
            m_negative: false,
            sigma: 10_000_000,
            onchain_timestamp: 0,
        };
        let now_ms = 1_950_000_000_000;
        let grid = compute_strike_grid(&oracle, &price, &svi, now_ms, 11, 10);
        assert_eq!(grid.strikes.len(), 11);
        // 中央が ATM 付近
        let mid = &grid.strikes[5];
        assert!((mid.strike_usd - grid.atm_strike_usd).abs() < 1e-6);
    }
}
