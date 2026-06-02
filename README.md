# PredictBot 🤖

Autonomous trading bot platform for **DeepBook Predict** on Sui blockchain.

> 🏆 Submission for [Sui Overflow 2026](https://overflow.sui.io/) — DeepBook Track

## 🎯 What It Does

PredictBot is a SaaS platform that lets users deploy capital into automated prediction market strategies on DeepBook Predict. Users select a strategy, deposit funds, and the bot handles execution, risk management, and reporting — all running on self-hosted infrastructure for low-latency execution.

## 🏗 Architecture

[ User Dashboard ]
        |
        v
[ Strategy Engine (Rust + Tokio) ]
        |
        v
[ DeepBook Predict Integration Layer ]
        |
        v
[ Self-hosted Sui Full Node ]

## 🛠 Tech Stack

- **Language:** Rust + Move
- **Async:** Tokio
- **Blockchain:** Sui (DeepBook Predict)
- **Infrastructure:** Self-hosted Sui full node

## 🔬 Status

🔨 Active development for Sui Overflow 2026 (May 7 - June 21, 2026)

### Development Phases

- **Phase 1** (Week 1): DeepBook Predict integration, core engine
- **Phase 2** (Week 2): Multi-strategy framework, UI
- **Phase 3** (Week 3): Polish, mainnet readiness, submission

## 🎓 Background

This project leverages infrastructure knowledge from prior work on Sui DeFi automation. The DeepBook Predict integration, strategy framework, user-facing platform, and Move contracts are all new development for Sui Overflow 2026.

## 📝 Author

**Iwao Kai** — Rust/Move Engineer | Self-hosted Sui Node Operator
- GitHub: [IWAOKAI](https://github.com/IWAOKAI)
- LinkedIn: [iwaokai](https://linkedin.com/in/iwaokai)

## 📄 License

MIT
