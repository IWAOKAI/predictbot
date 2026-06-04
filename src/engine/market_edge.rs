//! Market edge: DeepEdge の fair probability と、実際の市場 ask を比較。
//!
//! market ask は positions/minted から取得する。これは spread 込みの価格なので、
//! fair > market_ask が成立するときだけ「spread を超えた本物の edge」とみなす。

use std::collections::HashMap;

use crate::types::PositionMint;

/// (strike_i64, is_up) -> 最新 ask_probability の索引
pub struct MarketAskIndex {
    map: HashMap<(i64, bool), AskEntry>,
}

#[derive(Clone, Copy)]
struct AskEntry {
    ask_probability: f64,
    timestamp_ms: i64,
}

impl MarketAskIndex {
    /// 全 mint から特定 oracle のものだけ集め、(strike, is_up) ごとに最新 ask を採用
    pub fn from_mints(mints: &[PositionMint], oracle_id: &str) -> Self {
        let mut map: HashMap<(i64, bool), AskEntry> = HashMap::new();
        for m in mints.iter().filter(|m| m.oracle_id == oracle_id) {
            let key = (m.strike, m.is_up);
            let entry = AskEntry {
                ask_probability: m.ask_probability(),
                timestamp_ms: m.checkpoint_timestamp_ms,
            };
            map.entry(key)
                .and_modify(|e| {
                    if entry.timestamp_ms > e.timestamp_ms {
                        *e = entry;
                    }
                })
                .or_insert(entry);
        }
        Self { map }
    }

    /// strike(USD) と方向から最新 market ask 確率を引く
    /// strike_usd は 1e9 スケールの i64 に丸めて照合する
    pub fn lookup(&self, strike_usd: f64, is_up: bool) -> Option<f64> {
        let strike_i64 = (strike_usd * 1e9).round() as i64;
        self.map.get(&(strike_i64, is_up)).map(|e| e.ask_probability)
    }

    /// 最新 ask の timestamp も欲しい場合
    pub fn lookup_with_ts(&self, strike_usd: f64, is_up: bool) -> Option<(f64, i64)> {
        let strike_i64 = (strike_usd * 1e9).round() as i64;
        self.map
            .get(&(strike_i64, is_up))
            .map(|e| (e.ask_probability, e.timestamp_ms))
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// 1ストライク × 1方向の edge 判定
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectionalEdge {
    pub fair: f64,
    pub market_ask: Option<f64>,
    pub ev: Option<f64>,
    pub signal: EdgeSignal,
    pub market_ts_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeSignal {
    /// market データなし
    NoData,
    /// fair > market + threshold → 買い妙味
    Buy,
    /// fair < market - threshold → 割高、避ける
    Avoid,
    /// edge が閾値未満 → 中立
    Neutral,
}

/// EV のしきい値（spread を超えたと判断する最小 edge）。初期値 2%。
pub const EV_THRESHOLD: f64 = 0.02;

/// fair と market ask から DirectionalEdge を作る
pub fn evaluate_direction(
    fair: f64,
    market: Option<(f64, i64)>,
) -> DirectionalEdge {
    match market {
        None => DirectionalEdge {
            fair,
            market_ask: None,
            ev: None,
            signal: EdgeSignal::NoData,
            market_ts_ms: None,
        },
        Some((market_ask, ts)) => {
            let ev = fair - market_ask;
            let signal = if ev > EV_THRESHOLD {
                EdgeSignal::Buy
            } else if ev < -EV_THRESHOLD {
                EdgeSignal::Avoid
            } else {
                EdgeSignal::Neutral
            };
            DirectionalEdge {
                fair,
                market_ask: Some(market_ask),
                ev: Some(ev),
                signal,
                market_ts_ms: Some(ts),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mint(oracle: &str, strike: i64, is_up: bool, ask: i64, ts: i64) -> PositionMint {
        PositionMint {
            oracle_id: oracle.to_string(),
            predict_id: "p".into(),
            manager_id: "m".into(),
            trader: "t".into(),
            expiry: 0,
            strike,
            is_up,
            quantity: 1_000_000,
            cost: 0,
            ask_price: ask,
            checkpoint_timestamp_ms: ts,
        }
    }

    #[test]
    fn index_takes_latest_ask() {
        let mints = vec![
            mint("o1", 66_000_000_000_000, true, 500_000_000, 100),
            mint("o1", 66_000_000_000_000, true, 530_000_000, 200), // newer
            mint("o2", 66_000_000_000_000, true, 999_000_000, 300), // different oracle
        ];
        let idx = MarketAskIndex::from_mints(&mints, "o1");
        let ask = idx.lookup(66_000.0, true).unwrap();
        assert!((ask - 0.530).abs() < 1e-9, "should take latest, got {}", ask);
    }

    #[test]
    fn index_filters_by_oracle() {
        let mints = vec![mint("o2", 66_000_000_000_000, true, 999_000_000, 300)];
        let idx = MarketAskIndex::from_mints(&mints, "o1");
        assert!(idx.is_empty());
    }

    #[test]
    fn buy_signal_when_fair_exceeds_market() {
        // fair 0.62, market 0.55 → ev +0.07 > threshold → Buy
        let e = evaluate_direction(0.62, Some((0.55, 123)));
        assert_eq!(e.signal, EdgeSignal::Buy);
        assert!((e.ev.unwrap() - 0.07).abs() < 1e-9);
    }

    #[test]
    fn avoid_signal_when_market_expensive() {
        // fair 0.50, market 0.60 → ev -0.10 → Avoid
        let e = evaluate_direction(0.50, Some((0.60, 123)));
        assert_eq!(e.signal, EdgeSignal::Avoid);
    }

    #[test]
    fn no_data_when_market_absent() {
        let e = evaluate_direction(0.50, None);
        assert_eq!(e.signal, EdgeSignal::NoData);
        assert!(e.ev.is_none());
    }

    #[test]
    fn neutral_within_threshold() {
        // fair 0.51, market 0.50 → ev +0.01 < threshold → Neutral
        let e = evaluate_direction(0.51, Some((0.50, 123)));
        assert_eq!(e.signal, EdgeSignal::Neutral);
    }
}
