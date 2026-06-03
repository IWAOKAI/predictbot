//! SVI (Stochastic Volatility Inspired) モデル
//!
//! Gatheral 2004 の標準的な定式化:
//!   w(k) = a + b * (ρ * (k - m) + sqrt((k - m)^2 + σ^2))
//! ここで:
//!   k = log-moneyness = ln(K / F)
//!   w(k) = total implied variance (満期までの累積分散)
//!
//! DeepBook Predict の SVI イベントが返すパラメータは:
//!   a, b: u64 を 1e9 で割って f64
//!   rho:   u64 + rho_negative フラグで符号付き、1e9 でスケール
//!   m:     u64 + m_negative フラグで符号付き、1e9 でスケール
//!   sigma: u64 を 1e9 で割って f64（SVI の σ パラメータ、市場ボラとは別）

use crate::types::OracleSvi;

/// SVI パラメータを f64 に展開した形
#[derive(Debug, Clone, Copy)]
pub struct SviParams {
    pub a: f64,
    pub b: f64,
    pub rho: f64,
    pub m: f64,
    pub sigma: f64,
}

impl SviParams {
    pub fn from_event(svi: &OracleSvi) -> Self {
        // a と b は常に非負（SVI 制約）なのでそのまま 1e9 で割る
        let a = svi.a as f64 / 1e9;
        let b = svi.b as f64 / 1e9;

        // rho と m は符号付き
        let rho_mag = svi.rho as f64 / 1e9;
        let rho = if svi.rho_negative { -rho_mag } else { rho_mag };

        let m_mag = svi.m as f64 / 1e9;
        let m = if svi.m_negative { -m_mag } else { m_mag };

        let sigma = svi.sigma as f64 / 1e9;

        Self { a, b, rho, m, sigma }
    }

    /// log-moneyness k に対する total variance w(k) を返す
    /// 数学: w(k) = a + b * (ρ * (k - m) + sqrt((k - m)^2 + σ^2))
    pub fn total_variance(&self, k: f64) -> f64 {
        let dx = k - self.m;
        let radical = (dx * dx + self.sigma * self.sigma).sqrt();
        let w = self.a + self.b * (self.rho * dx + radical);
        // 数値上、負になる場合は 0 にクランプ（モデルとしては起きえないがガード）
        w.max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_params() -> SviParams {
        // 実際のテストネット観測値（svi イベントから）
        SviParams {
            a: 0.000036,
            b: 0.000609,
            rho: -0.943,
            m: 0.0199,
            sigma: 0.0100,
        }
    }

    #[test]
    fn total_variance_at_m_is_a_plus_b_sigma() {
        let p = sample_params();
        // k = m のとき: w = a + b * σ
        let w = p.total_variance(p.m);
        let expected = p.a + p.b * p.sigma;
        assert!((w - expected).abs() < 1e-12, "w={}, expected={}", w, expected);
    }

    #[test]
    fn total_variance_is_non_negative() {
        let p = sample_params();
        for &k in &[-0.1, -0.05, 0.0, 0.05, 0.1] {
            let w = p.total_variance(k);
            assert!(w >= 0.0, "w({})={} should be non-negative", k, w);
        }
    }
}
