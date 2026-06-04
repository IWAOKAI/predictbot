# DeepEdge Findings — Day 2 (2026-06-04)

## Calibration backtest (the real signal)

Source: 831 settled bets, 77 traders, extreme asks (<5%, >95%) removed.
Method: compare market ask probability to actual settlement outcome.
No look-ahead — uses only market ask (mint-time) vs realized outcome.

| Implied range | n   | implied | actual | gap    | ROI    |
|---------------|-----|---------|--------|--------|--------|
| 10-20%        | 26  | 15.4%   | 15.4%  | +0.0%  | +4%    |
| 20-30%        | 24  | 22.9%   | 25.0%  | +2.1%  | +15%   |
| 30-40%        | 42  | 35.6%   | 28.6%  | -7.1%  | -20%   |
| 40-50%        | 210 | 46.6%   | 35.7%  | -10.9% | -22%   |
| 50-60%        | 363 | 53.2%   | 54.8%  | +1.7%  | +3%    |
| 60-70%        | 41  | 64.1%   | 61.0%  | -3.1%  | -6%    |
| 70-80%        | 29  | 75.3%   | 86.2%  | +10.9% | +14%   |
| 80-90%        | 21  | 86.4%   | 90.5%  | +4.0%  | +5%    |
| 90-100%       | 34  | 92.1%   | 100.0% | +7.9%  | +9%    |

Overall: market-ask ROI -9% (spread cost). Mean abs calibration error 5.2%.

### Honest reading
- 50-60% (n=363, largest, most reliable): well-calibrated, gap +1.7%.
- 40-50% (n=210, reliable): market over-prices UP by ~11%.
- Favorites (70%+, small n): under-priced — consistent with the
  classic longshot-favorite bias, but small samples (n=21-34).
- Strong claims: only for 40-50% and 50-60% buckets (large n).
- Weak claims: tails (n<40) are suggestive, not conclusive.

## Accuracy backtest (Brier) — KNOWN LOOK-AHEAD, NOT a selling point

We computed DeepEdge Brier vs market Brier. Result looked spectacular
(0.0355 vs 0.2091, 83% improvement, 95% win rate) but this is a
look-ahead artifact:

- DeepEdge fair uses latest SVI/forward, observed ~5s before settlement.
- At T-5s the forward is already ~99.9% of settlement (verified:
  settlement $66,060 vs T-5s forward $65,995, diff $65 = 0.1%).
- So DeepEdge is "predicting" with the answer essentially in hand.
- Market ask reflects mint-time info (often tens of minutes earlier).

Conclusion: the Brier comparison measures SVI internal consistency,
NOT predictive skill. We keep the endpoint but flag it explicitly and
will NOT present it as evidence of predictive superiority.

A fair predictive comparison would require mint-time SVI, but the
public server only serves ~100 most-recent SVI/price events per oracle
(covering only the final minutes), so historical mint-time state is
unavailable. This is a data limitation we disclose openly.

## What this means for DeepEdge positioning

- Real edge: VISUALIZE market calibration + spread cost (-9% blind ROI),
  so users see where the market is reliable (50-60%) and where it is
  systematically off (40-50%).
- Not claiming "beat the market with math" on testnet data — that would
  be dishonest given look-ahead and small tail samples.
- Mainnet (Q3) with real participants will produce cleaner calibration
  data; DeepEdge is the tool that surfaces it.
