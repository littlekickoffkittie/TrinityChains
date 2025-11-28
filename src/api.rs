use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router, http::StatusCode, response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use chrono::Utc;

use crate::blockchain::{Blockchain, Block};
use crate::transaction::{Transaction, CoinbaseTx};
use crate::miner;
use crate::network::NetworkNode;
use crate::geometry::Coord;
use crate::crypto::KeyPair;

#[derive(Clone)]
pub struct Node {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub network: Arc<NetworkNode>,
    is_mining: Arc<AtomicBool>,
    blocks_mined: Arc<AtomicU64>,
    mining_task: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl Node {
    pub fn new(blockchain: Blockchain) -> Self {
        let blockchain_arc = Arc::new(RwLock::new(blockchain));
        let network_arc = Arc::new(NetworkNode::new(blockchain_arc.clone()));
        Self {
            blockchain: blockchain_arc,
            network: network_arc,
            is_mining: Arc::new(AtomicBool::new(false)),
            blocks_mined: Arc::new(AtomicU64::new(0)),
            mining_task: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start_mining(&self, miner_address: String) {
        if self.is_mining.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            println!("Mining is already in progress.");
            return;
        }

        let node_clone = self.clone();
        let task = tokio::spawn(async move {
            loop {
                if !node_clone.is_mining.load(Ordering::Relaxed) {
                    break;
                }

                let new_block = {
                    let bc = node_clone.blockchain.read().await;
                    if let Some(last_block) = bc.blocks.last() {
                        let transactions = bc.mempool.get_all_transactions();
                        let height = bc.blocks.len() as u64;
                        let reward = Blockchain::calculate_block_reward(height);
                        let coinbase_tx = Transaction::Coinbase(CoinbaseTx {
                            reward_area: Coord::from_num(reward),
                            beneficiary_address: miner_address.clone(),
                        });
                        let mut all_txs = vec![coinbase_tx];
                        all_txs.extend(transactions);

                        Some(Block::new(
                            height,
                            last_block.hash,
                            bc.difficulty,
                            all_txs,
                        ))
                    } else {
                        eprintln!("Cannot mine without a genesis block.");
                        None
                    }
                };

                if let Some(block) = new_block {
                    match miner::mine_block(block) {
                        Ok(mined_block) => {
                            let mut bc = node_clone.blockchain.write().await;
                            if bc.apply_block(mined_block.clone()).is_ok() {
                                node_clone.blocks_mined.fetch_add(1, Ordering::SeqCst);
                                node_clone.network.broadcast_block(&mined_block).await;
                                println!("Successfully mined and applied new block!");
                            } else {
                                eprintln!("Mined block was invalid or failed to apply.");
                            }
                        }
                        Err(e) => {
                            eprintln!("Mining failed: {}", e);
                        }
                    }
                } else {
                    // Stop mining if there's an issue creating a new block
                    break;
                }
            }
            // Ensure mining state is set to false when the loop exits
            node_clone.is_mining.store(false, Ordering::SeqCst);
            println!("Mining has stopped.");
        });

        *self.mining_task.write().await = Some(task);
    }

    pub async fn stop_mining(&self) {
        if self.is_mining.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            println!("Stopping mining...");
            if let Some(task) = self.mining_task.write().await.take() {
                task.abort();
                println!("Mining task aborted.");
            }
        } else {
            println!("Mining is not running.");
        }
    }
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub chain_height: u64,
    pub mempool_size: usize,
    pub blocks_mined: u64,
    pub difficulty: u64,
    pub total_earned: u64,
    pub current_reward: u64,
    pub avg_block_time: f64,
    pub uptime: u64,
    pub total_supply: u64,
    pub max_supply: u64,
    pub halving_era: u64,
    pub blocks_to_halving: u64,
}

#[derive(Serialize)]
pub struct BlocksResponse {
    pub blocks: Vec<Block>,
}

pub async fn run_api_server(node: Arc<Node>) {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    let api_routes = Router::new()
        .route("/blockchain/height", get(get_blockchain_height))
        .route("/blockchain/blocks", get(get_recent_blocks))
        .route("/blockchain/stats", get(get_blockchain_stats))
        .route("/transaction", post(submit_transaction))
        .route("/mining/start", post(start_mining))
        .route("/mining/stop", post(stop_mining))
        .route("/network/peers", get(get_peers))
        .route("/address/:addr/balance", get(get_address_balance))
        .route("/wallet/create", post(create_wallet))
        .with_state(node)
        .layer(cors.clone());

    let serve_dir = ServeDir::new("dashboard/dist");
    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(serve_dir)
        .layer(cors);

    let port = std::env::var("PORT").ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to port {}: {}", port, e);
            return;
        }
    };

    println!("API server listening on http://{}", addr);
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("API server error: {}", e);
    }
}

async fn get_blockchain_height(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let blockchain = node.blockchain.read().await;
    Json(blockchain.blocks.len() as u64)
}

async fn get_recent_blocks(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let blockchain = node.blockchain.read().await;
    let blocks: Vec<_> = blockchain.blocks.iter().rev().take(50).cloned().collect();
    Json(BlocksResponse { blocks })
}

async fn get_blockchain_stats(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let blockchain = node.blockchain.read().await;

    if blockchain.blocks.is_empty() {
        return Json(StatsResponse {
            chain_height: 0,
            mempool_size: 0,
            blocks_mined: 0,
            difficulty: 0,
            total_earned: 0,
            current_reward: 0,
            avg_block_time: 0.0,
            uptime: 0,
            total_supply: 0,
            max_supply: crate::blockchain::MAX_SUPPLY,
            halving_era: 0,
            blocks_to_halving: crate::blockchain::REWARD_HALVING_INTERVAL,
        });
    }

    let last_block = blockchain.blocks.last().unwrap(); // Safe to unwrap here
    let height = last_block.header.height;
    let total_supply = Blockchain::calculate_current_supply(height);

    // Calculate average block time from the last 100 blocks for more accuracy
    let window_size = 100.min(blockchain.blocks.len());
    let avg_block_time = if window_size > 1 {
        let last = last_block.header.timestamp;
        let first = blockchain.blocks[blockchain.blocks.len() - window_size].header.timestamp;
        (last - first) as f64 / (window_size - 1) as f64
    } else {
        0.0
    };

    let uptime = if let Some(genesis) = blockchain.blocks.first() {
        Utc::now().timestamp() - genesis.header.timestamp
    } else {
        0
    };

    let stats = StatsResponse {
        chain_height: height,
        mempool_size: blockchain.mempool.len(),
        blocks_mined: node.blocks_mined.load(Ordering::Relaxed),
        difficulty: blockchain.difficulty,
        total_earned: total_supply,
        current_reward: Blockchain::calculate_block_reward(height),
        avg_block_time,
        uptime: uptime as u64,
        total_supply,
        max_supply: crate::blockchain::MAX_SUPPLY,
        halving_era: blockchain.current_halving_era(),
        blocks_to_halving: blockchain.blocks_until_next_halving(),
    };
    Json(stats)
}

async fn submit_transaction(State(node): State<Arc<Node>>, Json(tx): Json<Transaction>) -> Response {
    let mut blockchain = node.blockchain.write().await;
    match blockchain.mempool.add_transaction(tx.clone()) {
        Ok(_) => {
            node.network.broadcast_transaction(&tx).await;
            (StatusCode::OK, Json("Transaction submitted successfully.")).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(format!("Failed to add transaction: {}", e))).into_response(),
    }
}

#[derive(Serialize, Deserialize)]
pub struct StartMiningRequest {
    pub miner_address: String,
}

async fn start_mining(State(node): State<Arc<Node>>, Json(req): Json<StartMiningRequest>) -> impl IntoResponse {
    node.start_mining(req.miner_address).await;
    (StatusCode::OK, Json("Mining started."))
}

async fn stop_mining(State(node): State<Arc<Node>>) -> impl IntoResponse {
    node.stop_mining().await;
    (StatusCode::OK, Json("Mining stopped."))
}

async fn get_peers(State(node): State<Arc<Node>>) -> impl IntoResponse {
    let peers = node.network.list_peers().await;
    Json(peers)
}

async fn get_address_balance(State(node): State<Arc<Node>>, Path(addr): Path<String>) -> impl IntoResponse {
    let blockchain = node.blockchain.read().await;
    let balance = blockchain.state.get_balance(&addr).to_num::<u64>();
    Json(BalanceResponse { balance })
}

#[derive(Serialize)]
struct WalletResponse {
    address: String,
    public_key: String,
}

async fn create_wallet() -> Response {
    match KeyPair::generate() {
        Ok(keypair) => {
            let response = WalletResponse {
                address: keypair.address(),
                public_key: hex::encode(keypair.public_key.serialize()),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Failed to generate keypair: {}", e))).into_response(),
    }
}
