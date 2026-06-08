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
