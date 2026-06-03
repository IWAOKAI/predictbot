# DeepEdge Development Progress

## Day 1 — 2026-06-03

### Engine & API
- HTTP server: Axum + Tokio, port 3000
- Routes: `/`, `/health`, `/api/markets`, `/api/markets/:id`, `/api/markets/:id/strikes`
- Public Predict Server client (status, oracles, oracle_state, vault, manager, pnl)
- Typed models with USD conversions (oracle prices 1e9, quote 1e6)

### Math
- SVI: `w(k) = a + b * (rho*(k-m) + sqrt((k-m)^2 + sigma^2))`
  - integer-to-f64 with signed rho/m
  - clamped non-negative
- Binary pricing (log-normal): `P(S_T > K) = N(d2)`, `d2 = (ln(F/K) - w/2) / sqrt(w)`
- Strike grid: forward rounded to tick, ATM ± N ticks, per-strike fair UP/DOWN
- 10 unit tests passing (SVI symmetry, ATM ≈ 0.5, ITM/OTM extremes, call+put = 1)

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
