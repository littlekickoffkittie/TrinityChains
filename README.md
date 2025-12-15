<div align="center">

<img src="documentation/assets/logo.png" alt="TrinityChain Logo" width="200"/>

![Trinity Chain](documentation/assets/text.png)

![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)
![Rust 1.70+](https://img.shields.io/badge/rust-1.70%2B-orange.svg)

**A blockchain where value is geometric area**

[Features](#features) • [Quick Start](#quick-start) • [Architecture](#architecture) • [API](#api) • [Contributing](#contributing)

</div>

---

## Overview

TrinityChain implements a proof-of-work blockchain where every UTXO is a triangle with real coordinates. The spendable value of each UTXO equals its geometric area, calculated using the Shoelace formula.

```
        △
       /│\
      / │ \     Area = 100.0 TRC
     /  │  \    
    /___│___\   
        │
        ↓
   Subdivide (Sierpiński)
        │
    ┌───┴───┐
    △   △   △
   
   33.3 + 33.3 + 33.3 = 100.0 TRC
```

**Current Status:** Functional implementation with test coverage, CLI tools, REST API, and web dashboard.
This project is actively maintained.

---

## Features

### Triangle-Based UTXO Model

| Operation | Input | Output | Mechanism |
|-----------|-------|--------|-----------|
| **Coinbase** | `∅` | `1△` | Mining creates new triangle |
| **Transfer** | `1△` | `1△` | Ownership change with geometric fee |
| **Subdivision** | `1△` | `3△` | Sierpiński fractal split |

### Geometric Fee Structure

Fees are deducted by reducing a triangle's area while preserving its coordinate identity:

```rust
// Fee: 0.1% of triangle area
Triangle { area: 100.0, vertices: [(0,0), (10,0), (5,10)] }
  ↓ [fee deduction]
Triangle { area: 99.9,  vertices: [(0,0), (10,0), (5,10)] }
```

### Technical Stack

| Component | Implementation |
|-----------|----------------|
| Consensus | SHA-256 PoW + Difficulty |
| Crypto | secp256k1 + BIP-39/BIP-32 |
| Storage | SQLite (deterministic) |
| Networking | TCP P2P + WebSocket bridge |
| Precision | I32F32 fixed-point (consensus) |

---

## Quick Start

### Prerequisites

```bash
# Install Rust 1.70+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify
rustc --version
```

### Build

```bash
git clone https://github.com/TrinityChain/TrinityChain.git
cd TrinityChain

cargo build --release
cargo test --lib
```

### Create Wallet

```bash
# Generate new wallet
./target/release/trinity new-wallet

# Output:
# Mnemonic: abandon ability able about above absent absorb...
# Address: tc1q3k2x9p7f8h5j6m2n4v8c9w1e3r5t7y9u0i8o6
```

### Run Node

```bash
# Start with default settings from config.toml
./target/release/trinity-node

# To connect to a peer
./target/release/trinity-node --peer <host:port>
```

### Mining

```bash
# Start the persistent miner
./target/release/trinity-miner

# To mine a single block
./target/release/trinity-mine-block
```

### Send Transaction

```bash
./target/release/trinity-send \
  tc1qRECIPIENT_ADDRESS \
  <triangle_hash> \
  "optional memo"
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                      TrinityChain Node                    │
├──────────────────────────────────────────────────────────┤
│                                                           │
│  ┌─────────────┐    ┌──────────────┐    ┌────────────┐ │
│  │   Wallet    │───▶│  Mempool     │───▶│   Miner    │ │
│  │  (secp256k1)│    │ (Pending TX) │    │ (SHA-256)  │ │
│  └─────────────┘    └──────────────┘    └────────────┘ │
│         │                   │                    │       │
│         │                   ▼                    ▼       │
│         │            ┌──────────────┐    ┌────────────┐ │
│         └───────────▶│  Blockchain  │◀───│   Block    │ │
│                      │  (UTXO Set)  │    │  (Header)  │ │
│                      └──────────────┘    └────────────┘ │
│                             │                    │       │
│                             ▼                    ▼       │
│                      ┌──────────────────────────────┐   │
│                      │   Persistence (SQLite)       │   │
│                      └──────────────────────────────┘   │
│                                                           │
├──────────────────────────────────────────────────────────┤
│  Network Layer: TCP P2P ◀──▶ WebSocket Bridge           │
├──────────────────────────────────────────────────────────┤
│  API Layer: REST Endpoints + WebSocket Subscriptions    │
└──────────────────────────────────────────────────────────┘
```

### Core Modules

```
src/
├── geometry.rs       # Triangle primitives, Shoelace area calculation
├── transaction.rs    # Coinbase, Transfer, Subdivision logic
├── blockchain.rs     # Chain validation, UTXO management, mempool
├── network.rs        # P2P message handling, peer discovery
├── miner.rs          # PoW mining, difficulty adjustment (10 blocks)
├── persistence.rs    # SQLite schema and queries
├── api.rs            # REST + WebSocket endpoints
├── crypto.rs         # secp256k1 signing and verification
├── wallet.rs         # UTXO selection, transaction building
└── hdwallet.rs       # BIP-39 mnemonic, BIP-32 derivation
```

### Concurrency Model

| Component | Synchronization | Reason |
|-----------|----------------|---------|
| P2P Network | `Arc<RwLock<T>>` | Multi-reader broadcast |
| API State | `Arc<Mutex<T>>` | Single-writer safety |
| Mining | `AtomicBool + AtomicU64` | Lock-free cancellation |

---

## API

### HTTP Endpoints

```
GET  /api/blockchain/stats              # Chain metrics
GET  /api/blockchain/blocks             # Recent blocks
GET  /api/blockchain/block/:hash        # Block details
GET  /api/address/:addr/balance         # Address balance + triangles
GET  /api/address/:addr/triangles       # Triangle coordinates
POST /api/transaction                   # Submit signed transaction
POST /api/mining/start                  # Begin mining
POST /api/mining/stop                   # Halt mining
GET  /api/mining/status                 # Hashrate, block count
GET  /api/network/peers                 # Connected peers

WebSocket
WS   /ws/p2p                            # P2P message bridge
```

### Example Usage

```bash
# Get blockchain statistics
curl http://localhost:3000/api/blockchain/stats | jq

# Query address balance
curl http://localhost:3000/api/address/tc1qYOUR_ADDRESS/balance

# Submit transaction (from wallet CLI)
curl -X POST http://localhost:3000/api/transaction \
  -H "Content-Type: application/json" \
  -d @transaction.json
```

---

## Dashboard

React-based monitoring interface with live WebSocket updates:

```bash
cd dashboard
npm install
npm run build
```

Access at `http://localhost:3000/dashboard`

**Features:**
- Real-time blockchain stats (height, difficulty, circulating supply)
- Block explorer with transaction details
- Mining control panel (start/stop, hashrate monitor)
- Network peer visualization
- Halving countdown

---

## Tokenomics

| Parameter | Value |
|-----------|-------|
| Initial Reward | 50 TRC + fees |
| Halving Interval | 210,000 blocks |
| Max Supply | 420,000,000 TRC |
| Block Time (target) | ~30 seconds |
| Current Difficulty | Dynamic (10-block avg) |

Supply curve follows geometric decay with periodic halvings.

---

## Documentation

| Document | Description |
|----------|-------------|
| [`ARCHITECTURE_MOC.md`](documentation/ARCHITECTURE_MOC.md) | Component map with ASCII diagrams |
| [`ARCHITECTURE_AUDIT.md`](documentation/ARCHITECTURE_AUDIT.md) | Data flow and module interactions |
| [`SAFETY_AUDIT.md`](documentation/SAFETY_AUDIT.md) | Concurrency and error handling review |
| [`TRIANGLE_UTXO_AUDIT.md`](documentation/TRIANGLE_UTXO_AUDIT.md) | UTXO model deep dive |
| [`API_ENDPOINTS.md`](documentation/API_ENDPOINTS.md) | Complete API reference |
| [`NODE_SETUP.md`](documentation/NODE_SETUP.md) | Production deployment guide |

---

## Deployment

Configurations for cloud deployment:

```
deployment/
├── render.yaml       # Render.com service config
└── vercel.json       # Vercel frontend config
```

See [`NODE_SETUP.md`](documentation/NODE_SETUP.md) for production deployment instructions.

---

## Contributing

This project needs developers, reviewers, and testers. Contributions are welcome.

### How to Contribute

```bash
# 1. Fork and clone
git clone https://github.com/YOUR_USERNAME/TrinityChain.git

# 2. Create feature branch
git checkout -b feature/your-feature

# 3. Make changes and test
cargo test --lib

# 4. Submit PR
git push origin feature/your-feature
```

### Areas Needing Work

- Security audits (cryptography, consensus)
- Performance optimization (mining, validation)
- Network protocol improvements
- Documentation and tutorials
- Test coverage expansion

Look for issues tagged `good first issue` or propose your own improvements.

### Code Standards

- Run `cargo fmt` and `cargo clippy` before committing
- Add tests for new functionality
- Document public APIs with rustdoc comments
- Follow existing module structure

---

## Project Structure

```
.
├── .github/          # CI/CD workflows
├── dashboard/        # React frontend
├── deployment/       # Cloud configs (Render, Vercel)
├── documentation/    # Architecture docs
├── scripts/          # Build and deployment scripts
├── src/              # Rust blockchain implementation
├── Cargo.toml
├── LICENSE
└── README.md
```

---

## Links

- **Repository:** https://github.com/TrinityChain/TrinityChain
- **Issues:** https://github.com/TrinityChain/TrinityChain/issues
- **License:** [MIT](LICENSE)

---

<div align="center">

**Built with Rust**

</div>
