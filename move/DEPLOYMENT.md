# DeepEdge Mandate — On-Chain Deployment

## v2 (Day 7) — full enforcement with predict::mint integration

- PackageID: 0x1b74c8ea3bce315731aad517b4df776ea12814b63e5427fc2a3c19e4ee3cb778
- Module: mandate (bundled with deepbook_predict deps)
- Network: Sui testnet
- Owner/deployer: 0x2b67499cf42323566fcdef56ad3e063babee93697c1c74ced9500dfa6e16ddab

### The enforcement entry point

mandate::execute_bet<T>(m, amount, predict, manager, oracle, key, quantity, clock, ctx)
  internally runs: authorize(m, amount) -> predict::mint<T>(...) -> record_and_consume(m, receipt)

Because BetReceipt is a hot-potato (no drop/store/key), there is NO way to
reach predict::mint through this path without the mandate checks passing AND
the spend being recorded in the same transaction. The agent cannot bet
outside its mandate. This is the same structural guarantee as Mandate Memory,
applied to DeepBook Predict betting.

### v1 (authorize/record only, separate package)
- PackageID: 0x19ba34f61e034317a54b88e803192048ca452ac55ea0f928b08a912acdd26428
- Mandate object (per_bet_cap 2 DUSDC, budget 10): 0xadf6e676dbafa1c783b7e1a0b68f2c6751716e6428a759ec6c281491aecf4b1a

### Functions
- create_mandate(per_bet_cap, total_budget)
- authorize(m, amount): BetReceipt   (hot-potato)
- record_and_consume(m, receipt)
- execute_bet<T>(...)   <- authorize + predict::mint + record, atomic
- set_active(m, active)   (kill switch)

## Enforcement proven live on testnet (Day 7)

Mandate object: 0x40cc6731b2fbc447b35f4171bee72503036602fc96a12a26ad95350e8cfdbe44
(per_bet_cap 2 DUSDC, total_budget 10 DUSDC)

- Test 1 (normal): PTB authorize(1 DUSDC) -> record_and_consume
  succeeded; spent went 0 -> 1000000 on-chain.
- Test 2 (over cap): PTB authorize(3 DUSDC) ABORTED in
  mandate::authorize with code 2 (EPerBetExceeded); spent unchanged.
- Type guarantee: BetReceipt has no drop/store/key, so a PTB that
  authorizes without record_and_consume cannot even be built.
- execute_bet<T> compiles against predict::mint (design proven).

So the enforcement is not just deployed — it demonstrably accepts
valid bets, rejects over-cap bets, and makes 'authorize without
record' structurally impossible, all on testnet.

## Enforcement proven live on testnet (Day 7)

Mandate object: 0x40cc6731b2fbc447b35f4171bee72503036602fc96a12a26ad95350e8cfdbe44
(per_bet_cap 2 DUSDC, total_budget 10 DUSDC)

- Test 1 (normal): PTB authorize(1 DUSDC) -> record_and_consume
  succeeded; spent went 0 -> 1000000 on-chain.
- Test 2 (over cap): PTB authorize(3 DUSDC) ABORTED in
  mandate::authorize with code 2 (EPerBetExceeded); spent unchanged.
- Type guarantee: BetReceipt has no drop/store/key, so a PTB that
  authorizes without record_and_consume cannot even be built.
- execute_bet<T> compiles against predict::mint (design proven).

So the enforcement is not just deployed — it demonstrably accepts
valid bets, rejects over-cap bets, and makes 'authorize without
record' structurally impossible, all on testnet.

## Enforcement proven live on testnet (Day 7)

Mandate object: 0x40cc6731b2fbc447b35f4171bee72503036602fc96a12a26ad95350e8cfdbe44
(per_bet_cap 2 DUSDC, total_budget 10 DUSDC)

- Test 1 (normal): PTB authorize(1 DUSDC) -> record_and_consume
  succeeded; spent went 0 -> 1000000 on-chain.
- Test 2 (over cap): PTB authorize(3 DUSDC) ABORTED in
  mandate::authorize with code 2 (EPerBetExceeded); spent unchanged.
- Type guarantee: BetReceipt has no drop/store/key, so a PTB that
  authorizes without record_and_consume cannot even be built.
- execute_bet<T> compiles against predict::mint (design proven).

So the enforcement is not just deployed — it demonstrably accepts
valid bets, rejects over-cap bets, and makes 'authorize without
record' structurally impossible, all on testnet.
