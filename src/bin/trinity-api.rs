//! Trinity API Server Binary
//!
//! Standalone HTTP API server for TrinityChain

use std::sync::Arc;
use trinitychain::api::{run_api_server, Node};
use trinitychain::blockchain::Blockchain;
use trinitychain::error::ChainError;

#[tokio::main]
async fn main() -> Result<(), ChainError> {
    // Initialize logging (if you have env_logger or similar)
    // env_logger::init();

    println!("🚀 Starting TrinityChain API Server...");

    // Create new blockchain (or load from persistence if you have that method)
    // For now, just create a new one
    let blockchain = Blockchain::new([0; 32], 1).unwrap();
    println!("✅ Initialized blockchain");

    // Create node
    let node = Arc::new(Node::new(blockchain));

    // Run API server
    println!("Starting API server...");
    if let Err(e) = run_api_server(node).await {
        eprintln!("❌ API server error: {}", e);
        return Err(ChainError::ApiError(format!("Server failed: {}", e)));
    }

    Ok(())
}
