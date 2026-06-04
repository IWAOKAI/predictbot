# DeepEdge Development Progress

## Day 1 — 2026-06-03

### Engine & API
- HTTP server: Axum + Tokio, port 3000
- Routes: `/`, `/health`, `/api/markets`, `/api/markets/:id`, `/api/markets/:id/strikes`
- Public Predict Server client (status, oracles, oracle_state, vault, manager, pnl)
- Typed models with USD conversions (oracle prices 1e9, quote 1e6)

### Math
- SVI: `w(k) = a + b * (rho*(k-m) + sqrt((k-m)^2 + sigma^2))`
- Binary pricing (log-normal): `P(S_T > K) = N(d2)`, `d2 = (ln(F/K) - w/2) / sqrt(w)`
- Strike grid: forward rounded to tick, ATM ± N ticks, per-strike fair UP/DOWN
- 10 unit tests passing (SVI symmetry, ATM ~ 0.5, ITM/OTM extremes, call+put = 1)

### Live data verified (testnet)
- 18 active BTC oracles, ~15 min cadence
- Sample at spot=$65,884, forward=$65,879:
  - ATM strike $65,879: fair_up 49.86%, fair_down 50.14%
  - -$250: fair_up 68.49%
  - +$250: fair_up 29.51%
  - IV (annualized): ~50%, plausible for BTC

### What v1 Edge Score does NOT yet do
- Does not compare fair probability against actual market ask_price
- "Edge" is therefore a market-quality indicator, not strict +EV
- Per-strike +EV requires aggregating `/positions/minted` per (oracle, strike, is_up)


## Day 2 — 2026-06-04

### Built
- Data layer: PositionMint type + positions_minted(limit) client method
  (discovered: full universe is 1,128 mints; no server-side oracle filter,
  no working pagination — limit caps at 1,128)
- MarketAskIndex: (strike, is_up) -> latest market ask
- Edge engine (/api/markets/:id/edges): per-strike fair vs market EV,
  Buy/Avoid/Neutral/NoData signals
- Calibration backtest (/api/backtest/calibration): market ask vs realized
  outcome, 10 probability buckets, UP/DOWN directional breakdown
- Accuracy backtest (/api/backtest/accuracy): Brier score — KEPT BUT
  DEMOTED due to self-identified look-ahead bias (see FINDINGS.md)

### Key findings (look-ahead free, see docs/FINDINGS.md)
- Blind market-ask betting: ROI -9% (spread cost)
- Strong UP participation bias (UP:DOWN ~= 7:3)
- 40-50% UP bets systematically over-priced: implied 46.7% vs actual
  34.0% (n=144, gap -12.7%)
- 50-60% band well-calibrated both directions (n=365)

### Honesty note
The Brier "83% improvement" was caught as a look-ahead artifact
(DeepEdge fair used near-expiry forward, ~99.9% of settlement).
Verified and documented rather than presented as predictive skill.

### Tests: 20 logic passing, 2 network ignored

### Tomorrow (Day 3)
- Frontend scaffold (Next.js) to visualize calibration + edge signals
- The "UP over-priced at 40-50%" finding is the headline visual

