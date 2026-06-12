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

impl SviParams {
    /// First derivative of total variance: w'(k) = b(rho + (k-m)/sqrt((k-m)^2 + sigma^2))
    pub fn w_prime(&self, k: f64) -> f64 {
        let dx = k - self.m;
        let radical = (dx * dx + self.sigma * self.sigma).sqrt();
        self.b * (self.rho + dx / radical)
    }

    /// Second derivative: w''(k) = b * sigma^2 / ((k-m)^2 + sigma^2)^(3/2)
    pub fn w_double_prime(&self, k: f64) -> f64 {
        let dx = k - self.m;
        let s2 = dx * dx + self.sigma * self.sigma;
        self.b * self.sigma * self.sigma / (s2 * s2.sqrt())
    }

    /// Gatheral's butterfly-arbitrage function. The surface is free of
    /// butterfly arbitrage at k iff g(k) >= 0:
    ///   g(k) = (1 - k w'/(2w))^2 - (w'^2/4)(1/w + 1/4) + w''/2
    pub fn butterfly_g(&self, k: f64) -> f64 {
        let w = self.total_variance(k);
        if w <= 0.0 {
            return f64::NEG_INFINITY; // degenerate surface
        }
        let wp = self.w_prime(k);
        let wpp = self.w_double_prime(k);
        let term1 = (1.0 - k * wp / (2.0 * w)).powi(2);
        let term2 = (wp * wp / 4.0) * (1.0 / w + 0.25);
        term1 - term2 + wpp / 2.0
    }
}

#[cfg(test)]
mod arbitrage_tests {
    use super::*;

    fn sample() -> SviParams {
        SviParams { a: 0.04, b: 0.4, rho: -0.4, m: 0.0, sigma: 0.1 }
    }

    #[test]
    fn w_prime_matches_finite_difference() {
        let p = sample();
        let h = 1e-6;
        for k in [-0.3, -0.1, 0.0, 0.05, 0.2] {
            let fd = (p.total_variance(k + h) - p.total_variance(k - h)) / (2.0 * h);
            assert!((p.w_prime(k) - fd).abs() < 1e-5, "w' mismatch at k={}", k);
        }
    }

    #[test]
    fn w_double_prime_matches_finite_difference() {
        let p = sample();
        let h = 1e-4;
        for k in [-0.3, -0.1, 0.0, 0.05, 0.2] {
            let fd = (p.total_variance(k + h) - 2.0 * p.total_variance(k)
                + p.total_variance(k - h)) / (h * h);
            assert!((p.w_double_prime(k) - fd).abs() < 1e-4, "w'' mismatch at k={}", k);
        }
    }

    #[test]
    fn sane_surface_is_butterfly_free_near_atm() {
        let p = sample();
        for i in -20..=20 {
            let k = i as f64 * 0.01;
            assert!(p.butterfly_g(k) >= 0.0, "g(k)<0 at k={} for a sane surface", k);
        }
    }
}

