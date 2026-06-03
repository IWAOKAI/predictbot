# DeepEdge 🎯

**Don't Bet Blind. See the Edge.**

A data-driven prediction market platform on DeepBook Predict (Sui).

> 🏆 Submission for [Sui Overflow 2026](https://overflow.sui.io/) — DeepBook Track

## 🎯 What It Does

DeepEdge empowers users to make informed bets on DeepBook Predict by surfacing the statistical edge of each prediction market. Instead of blind speculation or copy-trading, users see:

- Historical hit rates for similar conditions
- Oracle volatility analysis
- Liquidity depth metrics
- A clear Edge Score (0-100%) for each market

Users then make one-tap bets with full context — winning through intelligence, not luck.

## 🧠 Why DeepEdge?

Most DeFi tools turn users into passive participants:
- Bots that auto-trade for you (you lose when they lose)
- Copy-trading platforms (you blindly follow others)
- Yield farms (returns vanish over time)

DeepEdge takes a different approach: **arm users with information so they win through their own judgment**.

## 🏗 Architecture

```
[ User Wallet (Sui) ]
        |
        v
[ DeepEdge Web App (Next.js) ]
        |
        v
[ Edge Score Engine (Rust + Tokio) ]
        |
        v
[ Self-hosted Sui Full Node ]
        |
        v
[ DeepBook Predict ]
```

## 🛠 Tech Stack

- **Frontend:** Next.js 14, TailwindCSS, @mysten/dapp-kit
- **Backend:** Rust, Tokio, Axum
- **Smart Contracts:** Move (Sui)
- **Infrastructure:** Self-hosted Sui full node (1.8TB / 125GB RAM)
- **Data:** SQLite for historical market analysis

## 🎯 Edge Score Calculation

The core differentiator of DeepEdge is the Edge Score, calculated in real-time from:

- **Historical hit rate** (30%): Past performance under similar conditions
- **Oracle volatility** (25%): Asset price movement analysis
- **Liquidity depth** (15%): Market thickness and reliability
- **Time to expiry** (15%): Time decay considerations
- **Odds advantage** (15%): Current odds vs historical fair value

Edge Scores are computed using data streamed directly from a self-hosted Sui node, giving DeepEdge users a latency advantage over public RPC users.

## 🔬 Status

🔨 Active development for Sui Overflow 2026 (May 7 - June 21, 2026)

### Development Phases

- **Phase 1** (Week 1): DeepBook Predict data layer, Edge Score engine
- **Phase 2** (Week 2): Frontend, wallet integration, one-tap betting
- **Phase 3** (Week 3): Polish, mainnet readiness, submission

## 🎓 Background

This project leverages infrastructure knowledge from prior work on Sui DeFi automation, including a custom Sui full node operation running since DeepBook Predict's testnet launch. The Edge Score engine, frontend, betting logic, and Move contracts are all new development for Sui Overflow 2026.

## 📝 Author

**Iwao Kai** — Rust/Move Engineer | Self-hosted Sui Node Operator
- GitHub: [IWAOKAI](https://github.com/IWAOKAI)
- LinkedIn: [iwaokai](https://linkedin.com/in/iwaokai)

## 📄 License

MIT

## 🔌 Live API Endpoints (Day 1)

Running on port 3000:

- `GET /` — banner
- `GET /health` — service info JSON
- `GET /api/markets` — list of all DeepBook Predict BTC oracles
- `GET /api/markets/:oracle_id` — oracle detail with v1 Edge Score
- `GET /api/markets/:oracle_id/strikes?num=11&step=50` — per-strike fair UP/DOWN probabilities from SVI smile

Built on:
- Public Predict Server (testnet): `https://predict-server.testnet.mystenlabs.com`
- SVI total-variance model + log-normal binary pricing
