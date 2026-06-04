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

use crate::types::{Oracle, PositionMint, OracleSvi, OraclePrice};
use crate::engine::svi::SviParams;
use crate::engine::pricing::binary_call_probability;

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
    /// UP ベットのみの内訳
    pub up_count: usize,
    pub up_implied: f64,
    pub up_actual: f64,
    /// DOWN ベットのみの内訳
    pub down_count: usize,
    pub down_implied: f64,
    pub down_actual: f64,
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
    is_up: bool,
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
        is_up: mint.is_up,
    })
}

/// 方向別の (件数, 平均implied, 実勝率) を返す
fn direction_stats(bets: &[&&EvaluatedBet]) -> (usize, f64, f64) {
    let n = bets.len();
    if n == 0 {
        return (0, 0.0, 0.0);
    }
    let implied = bets.iter().map(|b| b.implied_prob).sum::<f64>() / n as f64;
    let wins = bets.iter().filter(|b| b.won).count();
    let actual = wins as f64 / n as f64;
    (n, implied, actual)
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

        // 方向別の内訳
        let ups: Vec<&&EvaluatedBet> = group.iter().filter(|b| b.is_up).collect();
        let downs: Vec<&&EvaluatedBet> = group.iter().filter(|b| !b.is_up).collect();
        let (up_count, up_implied, up_actual) = direction_stats(&ups);
        let (down_count, down_implied, down_actual) = direction_stats(&downs);

        buckets.push(CalibrationBucket {
            bucket_low: i as f64 / 10.0,
            bucket_high: (i as f64 + 1.0) / 10.0,
            bet_count: n,
            avg_implied_prob: avg_implied,
            actual_win_rate: win_rate,
            calibration_gap: gap,
            avg_roi,
            up_count,
            up_implied,
            up_actual,
            down_count,
            down_implied,
            down_actual,
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


/// DeepEdge の予測精度 vs 市場の予測精度（Brier Score 比較）
#[derive(Debug, Clone, Serialize)]
pub struct AccuracyReport {
    pub bets_evaluated: usize,
    /// 市場 ask（spread込み）を予測値とした Brier Score（低いほど正確）
    pub market_brier: f64,
    /// DeepEdge fair（満期直前SVI）を予測値とした Brier Score
    pub deepedge_brier: f64,
    /// DeepEdge が市場より正確だった割合（per-bet で |error| が小さかった率）
    pub deepedge_more_accurate_rate: f64,
    /// 改善率 = (market_brier - deepedge_brier) / market_brier
    pub brier_improvement_pct: f64,
    /// 注記
    pub caveat: String,
}

/// 1つの mint に対して、市場予測と DeepEdge 予測の二乗誤差を出す
struct AccuracySample {
    market_sq_error: f64,
    deepedge_sq_error: f64,
}

/// settled oracle の latest_svi と latest_price から、各 mint の予測精度を評価。
/// svi/price が無い oracle はスキップ。
fn evaluate_accuracy(
    mint: &PositionMint,
    settlement: i64,
    svi: &OracleSvi,
    price: &OraclePrice,
) -> Option<AccuracySample> {
    let implied = mint.ask_price as f64 / PRICE_DECIMALS;
    if implied < 0.05 || implied > 0.95 {
        return None;
    }
    let won = if mint.is_up {
        settlement > mint.strike
    } else {
        settlement <= mint.strike
    };
    let outcome = if won { 1.0 } else { 0.0 };

    // DeepEdge fair を SVI から計算
    let svi_params = SviParams::from_event(svi);
    let forward = price.forward_usd();
    let strike_usd = mint.strike as f64 / PRICE_DECIMALS;
    if forward <= 0.0 || strike_usd <= 0.0 {
        return None;
    }
    let log_moneyness = (strike_usd / forward).ln();
    let w = svi_params.total_variance(log_moneyness);
    let fair_up = binary_call_probability(forward, strike_usd, w);
    // mint の方向に合わせた DeepEdge 予測確率
    let deepedge_prob = if mint.is_up { fair_up } else { 1.0 - fair_up };

    let market_sq_error = (implied - outcome).powi(2);
    let deepedge_sq_error = (deepedge_prob - outcome).powi(2);

    Some(AccuracySample {
        market_sq_error,
        deepedge_sq_error,
    })
}

/// market ask vs DeepEdge fair の予測精度を比較
/// oracle_states: (oracle_id, settlement, svi, price) のタプル列が必要
pub fn run_accuracy(
    settlements: &std::collections::HashMap<String, (i64, OracleSvi, OraclePrice)>,
    mints: &[PositionMint],
) -> AccuracyReport {
    let mut samples: Vec<AccuracySample> = Vec::new();
    for m in mints {
        if let Some((settlement, svi, price)) = settlements.get(&m.oracle_id) {
            if let Some(s) = evaluate_accuracy(m, *settlement, svi, price) {
                samples.push(s);
            }
        }
    }

    let n = samples.len();
    if n == 0 {
        return AccuracyReport {
            bets_evaluated: 0,
            market_brier: 0.0,
            deepedge_brier: 0.0,
            deepedge_more_accurate_rate: 0.0,
            brier_improvement_pct: 0.0,
            caveat: "no samples".to_string(),
        };
    }

    let market_brier = samples.iter().map(|s| s.market_sq_error).sum::<f64>() / n as f64;
    let deepedge_brier = samples.iter().map(|s| s.deepedge_sq_error).sum::<f64>() / n as f64;
    let more_accurate = samples
        .iter()
        .filter(|s| s.deepedge_sq_error < s.market_sq_error)
        .count();
    let deepedge_more_accurate_rate = more_accurate as f64 / n as f64;
    let brier_improvement_pct = if market_brier > 0.0 {
        (market_brier - deepedge_brier) / market_brier * 100.0
    } else {
        0.0
    };

    AccuracyReport {
        bets_evaluated: n,
        market_brier,
        deepedge_brier,
        deepedge_more_accurate_rate,
        brier_improvement_pct,
        caveat: "LOOK-AHEAD BIAS: DeepEdge fair is computed from near-expiry SVI/forward (observed ~5s before settlement), while market ask reflects mint-time info (often tens of minutes earlier). The forward at T-5s is already ~99.9% of settlement, so this measures SVI internal consistency, NOT predictive skill. Do not cite as predictive superiority.".to_string(),
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
    fn direction_breakdown_splits_correctly() {
        let oracles = vec![oracle("o1", Some(66_060_000_000_000))];
        let mints = vec![
            // 50-55% UP, settlement 66060
            mint("o1", 66_000_000_000_000, true, 520_000_000),  // UP win
            mint("o1", 66_100_000_000_000, true, 530_000_000),  // UP loss
            // 50-55% DOWN
            mint("o1", 66_100_000_000_000, false, 510_000_000), // DOWN win
        ];
        let r = run_calibration(&oracles, &mints);
        let b = r.buckets.iter().find(|b| b.bucket_low == 0.5).unwrap();
        assert_eq!(b.up_count, 2, "two UP bets in 50-60 bucket");
        assert_eq!(b.down_count, 1, "one DOWN bet in 50-60 bucket");
        // UP: 1 win of 2 = 50%
        assert!((b.up_actual - 0.5).abs() < 1e-9);
        // DOWN: 1 win of 1 = 100%
        assert!((b.down_actual - 1.0).abs() < 1e-9);
    }

    #[test]
    fn unsettled_oracle_skipped() {
        let oracles = vec![oracle("o1", None)];
        let mints = vec![mint("o1", 66_000_000_000_000, true, 500_000_000)];
        let r = run_calibration(&oracles, &mints);
        assert_eq!(r.total_bets_evaluated, 0);
    }
}
