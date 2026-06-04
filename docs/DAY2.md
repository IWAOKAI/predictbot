# Day 2 plan — 2026-06-04

## 🎬 First thing tomorrow: Watch the workshop

**"How to Trade on DeepBook Predict"** (official Sui Overflow workshop)
URL: https://youtu.be/GncjVUEJw9Y

Notes to take while watching:
- [ ] How does the official workshop describe the user flow?
- [ ] Any API endpoints or arguments I missed?
- [ ] What does Mysten Labs emphasize as UX priorities?
- [ ] Any hints about mainnet timeline?
- [ ] Anything that contradicts my current DeepEdge design?

## Priority 1: True Edge = fair − market
Goal: each strike shows +EV/-EV vs actual market ask.

Steps:
1. Add `PredictServerClient::positions_minted_recent(oracle_id, limit)` filtering by oracle.
2. Build `market_ask_index`: for each `(strike, is_up)`, take latest ask_price.
3. Extend `StrikeFairProbability` with `market_up_ask`, `market_down_ask`, `ev_up`, `ev_down`.
4. New unit test using a real position mint sample.

## Priority 2: Edge Score v2
Goal: collapse per-strike EV into a single per-oracle "best edge available" number.
- score = max(ev_up, ev_down) over strikes, scaled to 0-100.
- Keep v1 as separate "market quality" metric.

## Priority 3: SQLite cache
- Schema: `oracles`, `prices`, `sviz`, `positions_minted`, `settled_results`.
- Background task: poll Predict Server every 30s, upsert.
- Endpoints read from cache when available.

## Priority 4 (stretch): Frontend scaffold
- `frontend/` Next.js 14 App Router.
- Tailwind, @mysten/dapp-kit, basic Markets List page consuming /api/markets.

## Other workshops noted
- OpenZeppelin Secure Move Patterns: June 5, 16:00 UTC (June 6, 01:00 JST). Skip live, wait for recording.
- Walrus Memory / Harbor: not relevant to DeepEdge.

## Open questions
- ask_bounds returns null on fresh oracles. When does it populate?
- Does the protocol charge a uniform spread, or per-strike? Inspect a settled oracle's mints.
- Edge against market is meaningful only if the market price is recent; how to define "recent"?

## DeepBook Sandbox (noted Day 1 late)

URL: github.com/MystenLabs/deepbook-sandbox

What it is: One-command Docker local env for DeepBook V3 (CLOB).
- Includes: token, deepbook (CLOB), margin, pyth, usdc
- Does NOT include: predict module (the prediction protocol DeepEdge uses)

Relevance to DeepEdge: LOW
- DeepBook Predict (binary prediction) is NOT in the sandbox
- Sandbox targets DeepBook V3 (orderbook) builders like Phasis
- DeepEdge can continue using the public testnet predict-server

Possible future uses (NOT priority):
1. If public testnet predict-server has downtime → spin up localnet
   and manually deploy predict-testnet-4-16 branch
2. If Phasis-style CLOB integration becomes part of DeepEdge later
3. Reference for understanding the broader DeepBook ecosystem

Decision: SKIP for now. Focus on Priority 1-4 (fair-vs-market, frontend, backtest).
Re-evaluate only if public predict-server becomes unreliable.
