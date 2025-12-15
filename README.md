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

### Run Tests

```bash
# Run all tests
cargo test

# Run wallet and transaction tests
cargo test --test wallet_and_transactions

# Run with output
cargo test -- --nocapture
```

**Test Coverage:**
- Wallet creation and persistence
- Transaction creation and validation
- Blockchain initialization
- Multi-wallet isolation
- Fee calculations
- Alice-to-Bob transfer demo

### Create Wallet

```bash
# Generate new wallet
./target/release/trinity-wallet new

# Show wallet address
./target/release/trinity-wallet address

# List all wallets
./target/release/trinity-wallet list

# Output example:
# Address: e54369c2ef44435ba34ef6ee881f33b2fa3126c0bf0e96a806ddc39ab91c046d
# Created: 2025-12-15T17:03:28.761074746+00:00
```

### Run Node

```bash
# Start a node with networking
./target/release/trinity-node

# To connect to a peer
./target/release/trinity-connect <peer_address:port>
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
# Send transaction to an address
./target/release/trinity-send <to_address> <amount> --from <wallet_name>

# Example:
./target/release/trinity-send 9448bf87d9a554a672ad99acef2a811b4f27f20568ea3d55c413b3ec4b0624d3 50 --from alice "Payment memo"
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
GET  /api/blockchain/height             # Current block height
GET  /api/blockchain/blocks             # Recent blocks
GET  /api/blockchain/block/:height      # Block details by height
GET  /api/blockchain/stats              # Chain metrics
GET  /api/address/:addr/balance         # Address balance
GET  /api/address/:addr/transactions    # Address transaction history
POST /api/transaction                   # Submit signed transaction
GET  /api/transaction/:hash             # Transaction details
GET  /api/mempool                       # Pending transactions
POST /api/mining/start                  # Begin mining
POST /api/mining/stop                   # Halt mining
GET  /api/mining/status                 # Mining status
GET  /api/network/peers                 # Connected peers
GET  /api/network/info                  # Network information
POST /api/wallet/create                 # Create new wallet
GET  /api/health                        # Health check
```

### Example Usage

```bash
# Get blockchain statistics
curl http://localhost:3000/api/blockchain/stats | jq

# Query address balance
curl http://localhost:3000/api/address/e54369c2ef44435ba34ef6ee881f33b2fa3126c0/balance

# Check mining status
curl http://localhost:3000/api/mining/status | jq
```

---

## Dashboard

Web-based monitoring interface with real-time updates:

```bash
# Access at http://localhost:3000
# The dashboard is served alongside the API server
./target/release/trinity-api
```

**Features:**
- Real-time blockchain stats (height, difficulty, transaction count)
- Block explorer with transaction details
- Mining control panel
- Network peer visualization

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

## Command Line Tools

| Binary | Purpose |
|--------|---------|
| `trinity-wallet` | Wallet creation and management |
| `trinity-send` | Send transactions between addresses |
| `trinity-balance` | Check address balance |
| `trinity-node` | Run a blockchain node |
| `trinity-miner` | Persistent background miner |
| `trinity-mine-block` | Mine a single block |
| `trinity-api` | Start REST API server |
| `trinity-server` | Start API server with P2P networking |
| `trinity-history` | View transaction history |
| `trinity-connect` | Connect to peer nodes |
| `trinity-addressbook` | Manage address book |
| `trinity-user` | Manage user profiles |

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

### Testing

We use integration tests to verify wallet and transaction functionality:

```bash
# Run the wallet and transaction test suite
cargo test --test wallet_and_transactions --verbose

# Expected output: 10 tests covering:
# ✓ Wallet creation
# ✓ Two-wallet scenarios (Alice & Bob)
# ✓ Wallet persistence (save/load)
# ✓ Keypair derivation
# ✓ Blockchain initialization
# ✓ Transfer transactions
# ✓ Multi-wallet isolation
# ✓ Fee calculations
```

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
├── tests/            # Integration tests
│   └── wallet_and_transactions.rs  # Wallet & transaction tests
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
