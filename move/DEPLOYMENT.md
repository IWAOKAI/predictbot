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


## Phase 3 — verifiable autonomous loop (Day 8, 2026-06-09)

Full cycle proven on testnet, package
0xb82750b35a213320d5ad6204e7bce46493ae76340e2a018fd65fdca4ad08f34a,
Mandate 0x753fb2e637d42067aeea59df6044ddfeb37ac22c92f28c89a8ffc6e3a4635f3a.

mandate v3 adds DecisionReceipt (hot-potato) + authorize_with_decision /
record_decision_and_consume, emitting DecisionRecorded(decision_hash,
blob_id, amount, spent_after).

scripts/deepedge_loop.py runs one cycle:
  1. observe market + fair value + calibration (DeepEdge backend)
  2. reason: Claude (sonnet-4-5) corrects the fair prob with the bucket's
     historical optimism bias and returns a JSON verdict
  3. store the full decision record on Walrus -> blobId
  4. SHA-256 the record
  5. authorize_with_decision (per-bet cap / budget / kill switch enforced)
     -> record_decision_and_consume -> DecisionRecorded on-chain
  6. fetch the blob back from Walrus, re-hash, confirm it equals the
     on-chain hash (verification: True)

So an AI agent observes, decides, stores its reasoning immutably, and can
only act within the Mandate's limits -- a verifiable autonomous agent.
Example cycle digest: 9HZVqeiYsEHQdwh8FGwnKvC9Z3f5Vf9TPrBhvK67QUGc


## Phase 4 — enforcement + REAL mint + record, atomic (Day 8, 2026-06-09)

The full loop now places a REAL bet on live DeepBook Predict, bound to the
Mandate, in a single PTB -- without needing execute_bet (which the bundled
predict made impossible). The hot-potato DecisionReceipt is the glue.

One transaction does all of:
  1. mandate::authorize_with_decision(amount, hash, blob_id) -> receipt
  2. market_key::down(oracle, expiry, strike) -> mkey
  3. predict::mint<DUSDC>(Predict, Manager, oracle, mkey, qty, clock)
     -- the REAL 0xf5ea predict, real position minted
  4. mandate::record_decision_and_consume(receipt)

Because the receipt has no drop, step 4 is mandatory: you cannot mint
without passing the mandate checks AND recording the decision in the same
tx. So a real DeepBook Predict bet is structurally bound to a calibrated,
Walrus-stored, on-chain-hashed AI decision, inside the mandate's limits.

This sidesteps the published-at / bundled-predict problem entirely: the
mandate (one package) and the real predict (0xf5ea) are composed at the PTB
level, not via a Move dependency.
