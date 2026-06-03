//! Binary option pricing under log-normal assumption
//!
//! 対数正規仮定で「forward F、満期 T、ストライク K のとき満期に S_T > K になる確率」を計算。
//!
//! 数式:
//!   d2 = (ln(F/K) - w/2) / sqrt(w)
//!   P(S_T > K) = N(d2)
//! ここで w = total variance (= σ_iv^2 * T)
//!
//! N(.) は標準正規分布の CDF（cumulative distribution function）。
//! statrs クレートで計算する。

use statrs::distribution::{ContinuousCDF, Normal};

/// 標準正規分布の CDF: N(x) = ∫_{-∞}^{x} φ(t) dt
/// φ(t) = exp(-t^2/2) / sqrt(2π)
fn standard_normal_cdf(x: f64) -> f64 {
    // mean=0, std=1 の正規分布
    Normal::new(0.0, 1.0)
        .expect("standard normal is always constructible")
        .cdf(x)
}

/// Binary call の公正確率 P(S_T > K)
///
/// 引数:
///   forward: フォワード価格 F (USD)
///   strike:  ストライク K (USD)
///   total_variance: w = σ_iv^2 * T (無次元)
///
/// 戻り値: 0.0〜1.0 の確率
pub fn binary_call_probability(forward: f64, strike: f64, total_variance: f64) -> f64 {
    if forward <= 0.0 || strike <= 0.0 {
        return 0.0;
    }
    if total_variance <= 0.0 {
        // 分散ゼロ = 確率的に揺れない = forward >= strike なら確実に上、そうでなければ確実に下
        return if forward >= strike { 1.0 } else { 0.0 };
    }
    let w = total_variance;
    let d2 = ((forward / strike).ln() - 0.5 * w) / w.sqrt();
    standard_normal_cdf(d2)
}

/// Binary put の公正確率 P(S_T <= K)
pub fn binary_put_probability(forward: f64, strike: f64, total_variance: f64) -> f64 {
    1.0 - binary_call_probability(forward, strike, total_variance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atm_call_is_approximately_half() {
        // F = K のとき、d2 = -w/(2*sqrt(w)) = -sqrt(w)/2
        // 短期の小さい w なら確率は 0.5 にかなり近い
        let p = binary_call_probability(66_000.0, 66_000.0, 0.0001);
        assert!((p - 0.5).abs() < 0.01, "ATM 1bp variance ≈ 0.5 got {}", p);
    }

    #[test]
    fn deep_itm_call_is_near_one() {
        // forward が strike をはるかに上回るとき、UP は ほぼ確実
        let p = binary_call_probability(70_000.0, 50_000.0, 0.0005);
        assert!(p > 0.95, "deep ITM should be >0.95, got {}", p);
    }

    #[test]
    fn deep_otm_call_is_near_zero() {
        let p = binary_call_probability(50_000.0, 70_000.0, 0.0005);
        assert!(p < 0.05, "deep OTM should be <0.05, got {}", p);
    }

    #[test]
    fn call_plus_put_equals_one() {
        let c = binary_call_probability(66_000.0, 65_500.0, 0.0003);
        let p = binary_put_probability(66_000.0, 65_500.0, 0.0003);
        assert!((c + p - 1.0).abs() < 1e-12);
    }

    #[test]
    fn zero_variance_collapses() {
        let p_up = binary_call_probability(66_000.0, 65_000.0, 0.0);
        assert_eq!(p_up, 1.0);
        let p_down = binary_call_probability(64_000.0, 65_000.0, 0.0);
        assert_eq!(p_down, 0.0);
    }
}
