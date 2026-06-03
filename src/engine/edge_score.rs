use serde::Serialize;

use crate::types::{Oracle, OraclePrice, OracleSvi};

/// Edge Score：0〜100 のスケール
/// 高いほど「統計的に informed な選択ができる」状態
#[derive(Debug, Clone, Serialize)]
pub struct EdgeScore {
    pub score: f64,
    pub direction: Direction,
    pub components: EdgeComponents,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// 市場は上昇に偏っている（UP がやや有利）
    Up,
    /// 市場は下落に偏っている（DOWN がやや有利）
    Down,
    /// 中立
    Neutral,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgeComponents {
    /// forward - spot の偏差（市場の方向感）
    pub forward_bias_pct: f64,
    /// SVI 由来のボラティリティ寄与
    pub volatility_factor: f64,
    /// SVI スキュー（rho）
    pub skew_factor: f64,
    /// 残り時間の品質（0.0〜1.0）
    pub time_quality: f64,
    /// 各要素の生スコア（0〜1）
    pub raw_scores: RawScores,
}

#[derive(Debug, Clone, Serialize)]
pub struct RawScores {
    pub directional: f64,
    pub volatility: f64,
    pub skew: f64,
    pub time: f64,
}

/// Edge Score の重み付け
const W_DIRECTIONAL: f64 = 0.40;
const W_VOLATILITY: f64 = 0.25;
const W_SKEW: f64 = 0.15;
const W_TIME: f64 = 0.20;

/// 1つの Oracle 状態から Edge Score を計算
pub fn compute_edge_score(
    oracle: &Oracle,
    price: &OraclePrice,
    svi: &OracleSvi,
    now_ms: i64,
) -> EdgeScore {
    // 1. Directional Edge（forward vs spot）
    let forward_bias = price.forward_bias(); // -inf..inf, 普通は -0.01..0.01 くらい
    let directional_raw = (forward_bias.abs() * 100.0).min(1.0); // 1% の乖離で max
    let direction = if forward_bias > 0.0005 {
        Direction::Up
    } else if forward_bias < -0.0005 {
        Direction::Down
    } else {
        Direction::Neutral
    };

    // 2. Volatility Quality
    // sigma が極端に高い = 予測困難（スコア下げる）
    // sigma が低すぎる = 動きなく機会なし（スコア下げる）
    // 適度な値を理想とする
    let sigma = svi.sigma_f();
    let volatility_raw = volatility_quality_score(sigma);

    // 3. Skew（rho）
    // |rho| が大きい = 方向への偏り強い → informed bet ありえる
    let rho_abs = svi.rho_signed().abs();
    let skew_raw = (rho_abs * 2.0).min(1.0);

    // 4. Time Quality
    // 残り時間が長すぎても短すぎても情報優位は薄い
    let time_pct = oracle.time_remaining_pct(now_ms);
    let time_raw = time_quality_score(time_pct);

    let score = (directional_raw * W_DIRECTIONAL
        + volatility_raw * W_VOLATILITY
        + skew_raw * W_SKEW
        + time_raw * W_TIME)
        * 100.0;
    let score = score.clamp(0.0, 100.0);

    EdgeScore {
        score,
        direction,
        components: EdgeComponents {
            forward_bias_pct: forward_bias * 100.0,
            volatility_factor: sigma,
            skew_factor: svi.rho_signed(),
            time_quality: time_pct,
            raw_scores: RawScores {
                directional: directional_raw,
                volatility: volatility_raw,
                skew: skew_raw,
                time: time_raw,
            },
        },
    }
}

/// sigma が適度な値の時に高スコアを返す
/// sigma 0.001..0.01 で最大付近、それ以外は減衰
fn volatility_quality_score(sigma: f64) -> f64 {
    let ideal = 0.003; // 経験則の初期値、Day 2 でキャリブレーション
    let dist = (sigma - ideal).abs();
    let score = (-dist * 200.0).exp();
    score.clamp(0.0, 1.0)
}

/// 残り時間が 30〜70% の時に高スコア（ど真ん中が最も情報優位）
fn time_quality_score(time_pct: f64) -> f64 {
    // 中央 0.5 で max、両端で 0 に近づく
    let centered = (time_pct - 0.5).abs() * 2.0; // 0.0..1.0
    (1.0 - centered).max(0.0)
}
