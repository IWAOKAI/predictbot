//! Calibration backtest: 過去の settled oracle に対する mint を集計し、
//! 「市場が付けた確率（ask）」が実際の結果をどれだけ正確に予測したかを測る。
//!
//! 各 mint:
//!   - is_up, strike, ask_price, oracle_id
//! 各 oracle:
//!   - settlement_price（settled のみ）
//!
//! 勝敗:
//!   won = if is_up { settlement > strike } else { settlement <= strike }
//!   payout = if won { 1.0 } else { 0.0 }
//!   cost   = ask_price / 1e9   (= 市場が見た確率 = $1 当たり支払い)
//!   roi    = (payout - cost) / cost

use std::collections::HashMap;
use serde::Serialize;

use crate::types::{Oracle, PositionMint};

const PRICE_DECIMALS: f64 = 1e9;

/// 1つの確率バケット（例: 50-60%）の集計
#[derive(Debug, Clone, Serialize)]
pub struct CalibrationBucket {
    /// バケット下限（0.0, 0.1, ... 0.9）
    pub bucket_low: f64,
    pub bucket_high: f64,
    pub bet_count: usize,
    /// 市場が見た平均確率（このバケットの ask 平均）
    pub avg_implied_prob: f64,
    /// 実際の勝率
    pub actual_win_rate: f64,
    /// キャリブレーション誤差 = actual - implied（正なら市場が過小評価）
    pub calibration_gap: f64,
    /// このバケットを ask で全部買ったときの平均 ROI
    pub avg_roi: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalibrationReport {
    pub total_settled_oracles: usize,
    pub total_bets_evaluated: usize,
    pub overall_win_rate: f64,
    pub overall_avg_implied: f64,
    /// 全ベットを市場 ask で買ったときの平均 ROI（市場効率性の指標）
    pub overall_avg_roi: f64,
    /// 市場の総合キャリブレーション誤差（|actual - implied| の加重平均）
    pub mean_abs_calibration_error: f64,
    pub buckets: Vec<CalibrationBucket>,
}

struct EvaluatedBet {
    implied_prob: f64,
    won: bool,
    roi: f64,
}

/// settled oracle の settlement_price を引く索引を作る
fn settlement_index(oracles: &[Oracle]) -> HashMap<String, i64> {
    oracles
        .iter()
        .filter(|o| o.settlement_price.is_some())
        .map(|o| (o.oracle_id.clone(), o.settlement_price.unwrap()))
        .collect()
}

/// 1つの mint を評価。settlement が無ければ None。
fn evaluate_bet(mint: &PositionMint, settlement: i64) -> Option<EvaluatedBet> {
    let implied = mint.ask_price as f64 / PRICE_DECIMALS;
    // ask が極端（< 5% or > 95%）な mint は除外。
    // これらは「ほぼ確実な賭け」で、市場効率性の分析にノイズを与える
    // （ロングショットや本命の偏りで calibration が歪むため）。
    if implied < 0.05 || implied > 0.95 {
        return None;
    }
    let won = if mint.is_up {
        settlement > mint.strike
    } else {
        settlement <= mint.strike
    };
    let payout = if won { 1.0 } else { 0.0 };
    let roi = (payout - implied) / implied;
    Some(EvaluatedBet {
        implied_prob: implied,
        won,
        roi,
    })
}

/// メインのバックテスト関数
pub fn run_calibration(oracles: &[Oracle], mints: &[PositionMint]) -> CalibrationReport {
    let settle = settlement_index(oracles);

    let mut evaluated: Vec<EvaluatedBet> = Vec::new();
    for m in mints {
        if let Some(&s) = settle.get(&m.oracle_id) {
            if let Some(b) = evaluate_bet(m, s) {
                evaluated.push(b);
            }
        }
    }

    // 10 バケット (0-10%, ..., 90-100%)
    let mut bucket_bets: Vec<Vec<&EvaluatedBet>> = vec![Vec::new(); 10];
    for b in &evaluated {
        let idx = ((b.implied_prob * 10.0).floor() as usize).min(9);
        bucket_bets[idx].push(b);
    }

    let mut buckets = Vec::new();
    let mut weighted_abs_error = 0.0;
    for (i, group) in bucket_bets.iter().enumerate() {
        if group.is_empty() {
            continue;
        }
        let n = group.len();
        let wins = group.iter().filter(|b| b.won).count();
        let win_rate = wins as f64 / n as f64;
        let avg_implied = group.iter().map(|b| b.implied_prob).sum::<f64>() / n as f64;
        let avg_roi = group.iter().map(|b| b.roi).sum::<f64>() / n as f64;
        let gap = win_rate - avg_implied;
        weighted_abs_error += gap.abs() * n as f64;
        buckets.push(CalibrationBucket {
            bucket_low: i as f64 / 10.0,
            bucket_high: (i as f64 + 1.0) / 10.0,
            bet_count: n,
            avg_implied_prob: avg_implied,
            actual_win_rate: win_rate,
            calibration_gap: gap,
            avg_roi,
        });
    }

    let total = evaluated.len();
    let overall_win_rate = if total > 0 {
        evaluated.iter().filter(|b| b.won).count() as f64 / total as f64
    } else {
        0.0
    };
    let overall_avg_implied = if total > 0 {
        evaluated.iter().map(|b| b.implied_prob).sum::<f64>() / total as f64
    } else {
        0.0
    };
    let overall_avg_roi = if total > 0 {
        evaluated.iter().map(|b| b.roi).sum::<f64>() / total as f64
    } else {
        0.0
    };
    let mean_abs_calibration_error = if total > 0 {
        weighted_abs_error / total as f64
    } else {
        0.0
    };

    let settled_count = settle.len();

    CalibrationReport {
        total_settled_oracles: settled_count,
        total_bets_evaluated: total,
        overall_win_rate,
        overall_avg_implied,
        overall_avg_roi,
        mean_abs_calibration_error,
        buckets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OracleStatus;

    fn oracle(id: &str, settlement: Option<i64>) -> Oracle {
        Oracle {
            predict_id: "p".into(),
            oracle_id: id.into(),
            oracle_cap_id: "c".into(),
            underlying_asset: "BTC".into(),
            expiry: 1000,
            min_strike: 50_000_000_000_000,
            tick_size: 1_000_000_000,
            status: if settlement.is_some() { OracleStatus::Settled } else { OracleStatus::Active },
            activated_at: 0,
            settlement_price: settlement,
            settled_at: settlement.map(|_| 1000),
            created_checkpoint: 0,
        }
    }

    fn mint(oid: &str, strike: i64, is_up: bool, ask: i64) -> PositionMint {
        PositionMint {
            oracle_id: oid.into(),
            predict_id: "p".into(),
            manager_id: "m".into(),
            trader: "t".into(),
            expiry: 1000,
            strike,
            is_up,
            quantity: 1_000_000,
            cost: 0,
            ask_price: ask,
            checkpoint_timestamp_ms: 0,
        }
    }

    #[test]
    fn correct_win_loss_judgment() {
        // settlement = 66,060. strike 66,084 DOWN -> 66060 <= 66084 -> DOWN won
        let s = 66_060_000_000_000;
        let m = mint("o1", 66_084_000_000_000, false, 552_000_000);
        let b = evaluate_bet(&m, s).unwrap();
        assert!(b.won, "DOWN below strike should win");
        // payout 1.0, cost 0.552 -> roi positive
        assert!(b.roi > 0.0);
    }

    #[test]
    fn up_loss_when_settlement_below_strike() {
        let s = 66_060_000_000_000;
        let m = mint("o1", 66_212_000_000_000, true, 585_000_000);
        let b = evaluate_bet(&m, s).unwrap();
        assert!(!b.won, "UP below strike should lose");
        assert!((b.roi - (-1.0)).abs() < 1e-9, "loss roi = -1.0");
    }

    #[test]
    fn extreme_ask_rejected() {
        let s = 66_060_000_000_000;
        // 4% は除外
        assert!(evaluate_bet(&mint("o1", 66_000_000_000_000, true, 40_000_000), s).is_none());
        // 96% は除外
        assert!(evaluate_bet(&mint("o1", 66_000_000_000_000, true, 960_000_000), s).is_none());
        // 50% は通る
        assert!(evaluate_bet(&mint("o1", 66_000_000_000_000, true, 500_000_000), s).is_some());
    }

    #[test]
    fn report_aggregates_buckets() {
        let oracles = vec![oracle("o1", Some(66_060_000_000_000))];
        let mints = vec![
            mint("o1", 66_084_000_000_000, false, 552_000_000), // DOWN win, ask .552
            mint("o1", 66_212_000_000_000, true, 585_000_000),  // UP loss, ask .585
            mint("o1", 66_564_000_000_000, false, 788_000_000), // DOWN win, ask .788
        ];
        let r = run_calibration(&oracles, &mints);
        assert_eq!(r.total_bets_evaluated, 3);
        assert_eq!(r.total_settled_oracles, 1);
        // 2 wins of 3
        assert!((r.overall_win_rate - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn unsettled_oracle_skipped() {
        let oracles = vec![oracle("o1", None)];
        let mints = vec![mint("o1", 66_000_000_000_000, true, 500_000_000)];
        let r = run_calibration(&oracles, &mints);
        assert_eq!(r.total_bets_evaluated, 0);
    }
}
