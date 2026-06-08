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


## Day 3 — 2026-06-05

### Built
- Wallet connection via @mysten/dapp-kit
  - Providers (SuiClient + Wallet + QueryClient) loaded client-side
    only via dynamic import (ssr: false) — avoids React Context SSR
    crash in Next.js build
  - ConnectButton + WalletBar (shows connected address) on markets page
  - Verified: Slush wallet connects, address displays

### Researched (betting flow groundwork)
- mint tx is a 2-step PTB:
  1. market_key::up/down(oracle_id: ID, expiry: u64, strike: u64) -> MarketKey
  2. predict::mint<DUSDC>(Predict mut, PredictManager mut, OracleSVI ref,
     MarketKey, quantity: u64, Clock ref, ctx)
  - No explicit coin: payment is drawn from PredictManager's internal balance
- PredictManager is a SHARED object with an `owner` field
  - Discover via GET /managers?owner={address} -> returns manager_id
  - count==0 means user needs to create one
- Our testnet manager 0x870882... has balance_manager.balances.size=0
  -> must deposit DUSDC before betting; positions.size=1 (the old -$0.90)

### Remaining for full betting (next session, ~1 day)
1. Backend endpoint to proxy /managers?owner=
2. PredictManager discovery UI + "create manager" tx for new users
3. DUSDC faucet/source investigation + deposit tx
4. mint tx (2-step PTB) + sign/send/result
5. Slush must be switched to TESTNET before any real bet

### Tunnel stability note (IMPORTANT)
Repeated "Too many open files" crashes came from stray SSH tunnels
auto-retrying. Next time, on the Mac, use the SAFE tunnel command:
  ssh -o ServerAliveInterval=60 -o ExitOnForwardFailure=yes -N \
      -L 3001:localhost:3001 -L 3000:localhost:3000 root@SERVER
The -N (no shell) and ExitOnForwardFailure=yes (fail fast on port
conflict) prevent the runaway loop.


## Day 4 — 2026-06-05 (continued)

### MILESTONE: betting works end-to-end on testnet
Verified real on-chain transactions via Slush wallet (testnet):
- deposit $5 into PredictManager -> balance updated live ($0 -> $5)
- mint UP bet $2 -> succeeded, balance $5 -> $4.03
- mint DOWN bet -> succeeded
The full loop is live: connect wallet -> see SVI fair value ->
choose direction/strike -> one-tap bet -> on-chain mint.

### Backend
- /api/manager?owner= endpoint (proxies predict-server /managers?owner=)
  -> returns manager_id, or empty for new users

### Frontend
- components/BetPanel.tsx: PredictManager discovery, DUSDC deposit,
  UP/DOWN, strike select, bet amount, wallet signing
  (useSignAndExecuteTransaction)
- lib/transactions.ts: buildDepositTx (coinWithBalance),
  buildMintTx (2-step PTB: market_key::up|down -> predict::mint<DUSDC>)
- tsconfig target -> es2020 (BigInt for u64 amounts)

### UX fixes from self-testing
- bet button 'BET !' with a CONFIRM step (yellow panel: amount +
  direction + strike + fair value + "this will sign an on-chain tx")
  -> prevents accidental one-click bets, safe for demo
- fair value box explains meaning ("model probability BTC settles
  above/below this strike; bet when market price < fair")
- 'ATM' relabeled 'current price' (non-finance users)
- fairPct falls back to nearest strike (fixed '-%')

### Debugging lessons (cost real time)
- NEVER write .tsx via `cat > file << EOF`: heredoc silently dropped
  a line (missing <a tag) AND the useEffect opening line, causing
  cascading SWC "Expected jsx identifier" errors that pointed at the
  WRONG line. Always write .tsx via Python open(w).
- `npm run build` then `next dev` corrupts the shared .next cache
  ("Cannot find module './948.js'"). After any build, rm -rf .next
  before dev. For UI tweaks, rely on dev hot-reload, don't run build.
- SSH tunnel: use the safe command (ServerAliveInterval=60,
  ExitOnForwardFailure=yes, -N) to avoid fd-exhaustion runaway.

### Remaining (next session)
- TODO: market-near-expiry display (0m markets show extreme 100%/0%
  fair, looks odd in demo) -- code locations found:
  app/page.tsx timeUntil() line 8, market page timeStr line 52-53
- TODO: real-time auto-refresh (30s)
- TODO: confirm-before-bet is done; consider bet history (positions)
- Demo video (~Day 15), DeepSurge writeup upgrade (submission week)


## Day 5 — 2026-06-05 (continued, same long session)

### Added features
- My Positions panel (components/MyPositions.tsx): connected wallet's
  bet history via /api/manager/positions + /api/manager/summary
  (UP/DOWN, strike, fill %, cost, date, suivision tx link; account
  value + realized P&L header). Scoped to current market on detail
  page, all-markets on portfolio page.
- /portfolio page: all bets in one place + nav link
- /overview page: DeepEdge fair value across every live BTC market in
  one sortable table (asset, expiry, spot, ATM fair UP/DOWN, IV),
  closing-soon flagged, rows link to detail + nav link
- Backend: /api/manager/positions, /api/manager/summary endpoints
- IV explained for non-experts (header subtitle 'expected swing' +
  plain-language SVI chart caption)
- ATM relabeled '(current price)' in the bet strike selector

### Nav is now: Markets · Overview · Insights · Portfolio

### Key data finding (shapes strategy)
- /api/markets returns 3647 markets total, all BTC; only ~19 are
  status=active at any time (rest settled).
- Investigated market_data_points across all active markets for an
  "edge ranking" feature: nearly all have 0-3 points, best_edge is
  null everywhere. CONCLUSION: real fair-vs-market edge ranking is
  not possible on testnet (too little live order-flow). Deferred.
  On mainnet (Q3) this becomes viable. For now the math we can show
  is fair value + SVI smile + calibration, all of which we do.

### Debugging lesson reinforced
- '<' and '>' also get silently dropped when writing .tsx (parsed as
  JSX). The reduce comparison 'Math.abs(a) < Math.abs(b)' lost its
  '<'. Same class as the earlier missing '<a'. ALWAYS grep for the
  operator/opening-tag after writing .tsx via Python.

### State: feature-complete enough to win
Markets list, market detail (SVI smile + fair + 30s live), Insights
(calibration), real betting (deposit + mint + confirm), My Positions,
Portfolio, Overview. Real testnet bets placed and shown.

### NEXT PHASE = communicate, not build
- README (judges' first impression) -- HIGH
- demo video script + recording (~Day 13-15) -- HIGH
- DeepSurge writeup upgrade (submission week 6/19-21) -- HIGH
- minor polish only if needed; further features are low priority

## Day 6 — 2026-06-06

### Shifted from BUILD to COMMUNICATE (feature set complete)

### Verified demo still works (all 4 pillars)
- backend/frontend 200; testnet bet history intact (3 mints)
- overview, market detail fair calc, positions, portfolio summary
  (account ~$4.7, realized -$0.90) all OK
- active markets fluctuate ~8-19; /api/markets returns 3647 total

### Checked vs official Problem Statement
- fits 'Analytics & tooling', overlaps idea #9 but adds real
  betting -> differentiated; all min requirements met

### Decided NOT to build (fact-checked)
- Leverage: predict has NO margin/leverage fns (all 17 modules
  checked). Deferred / not feasible.
- Edge ranking: testnet order-flow too thin (best_edge null).
- AI copy-trader / Walrus advisor log: parked post-hackathon.

### README fully rewritten (the win today)
- Replaced stale Day-1 Edge Score fiction with shipped reality.
- problem / what it does / IT ACTUALLY WORKS (+flow, real txns) /
  HONEST ANALYSIS (-12.7% UP + look-ahead disclosure) / five
  screens / tech / API / mainnet roadmap
- 4 screenshots in docs/screenshots/, embedded + pushed

### Lesson
- With a complete product, communicating beats building.
- Writing .tsx/.md via heredoc drops chars (<a, <, code fences);
  use Python line-lists with self-checks.

### NEXT
- demo video (~Day 13-15), DeepSurge writeup (6/19-21),
  optional 'How to run' in README

## Day 7 — 2026-06-07

### Morning: communicate phase
- DeepSurge submission draft written (docs/DEEPSURGE.md, gitignored
  until submission week)
- README: added 'longer-term vision: a verifiable AI layer' section

### Afternoon: went ambitious — on-chain enforcement
Saw competitor 'Mandate Memory' (AI + Walrus + enforcement, already
built). Decided NOT to copy it but to add a real enforcement layer to
DeepEdge. Verified Walrus works over plain HTTP (PUT/GET blobs).

Built and deployed an on-chain Mandate enforcement contract (Move):
- Mandate object: per-bet cap, total budget, spent, kill switch
- BetReceipt hot-potato (no drop/store/key)
- execute_bet<T>: authorize -> predict::mint -> record, atomic
  => agent cannot reach mint without passing checks AND recording
- testnet package: 0x1b74c8ea3bce315731aad517b4df776ea12814b63e5427fc2a3c19e4ee3cb778
- v1 (authorize/record only): 0x19ba34f6...; Mandate obj 0xadf6e676...

### Hard-won lessons (Move dependency hell)
- Sui/MoveStdlib version conflicts -> pin both to the rev the git dep
  uses (b38bca) with override, OR let new CLI's environment resolve it
- Had to upgrade Sui CLI 1.67.3 -> 1.73.1 (protocol 115 vs 125); old
  CLI couldn't handle the use_environment=testnet scheme
- 'unpublished dependencies' + 'already published' -> remove the
  [published.testnet] entry from Published.toml, then publish with
  --with-unpublished-dependencies (bundles predict deps into the pkg)

### NEXT (the ambitious build continues)
- Phase 2: AI reasoning (Claude reads fair value + calibration)
- Phase 3: log decisions to Walrus, hash on-chain
- Phase 4: actually CALL execute_bet in a PTB (prove enforcement live)
- Phase 5: surface in frontend / demo
- KEEP DeepEdge core (5 screens, README) intact; demo video; submit 6/21
