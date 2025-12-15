//! Network node for TrinityChain - TUI Edition

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::env;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use trinitychain::api::Node; // Import the unified Node
use trinitychain::blockchain::Blockchain;
use trinitychain::config::load_config;
use trinitychain::persistence::Database;

#[derive(Clone)]
struct NodeStats {
    chain_height: u64,
    peer_count: usize,
    uptime_secs: u64,
    status: String,
    peers: Vec<String>,
    last_block_hash: String,
}

impl Default for NodeStats {
    fn default() -> Self {
        Self {
            chain_height: 0,
            peer_count: 0,
            uptime_secs: 0,
            status: "Initializing...".to_string(),
            peers: Vec::new(),
            last_block_hash: "N/A".to_string(),
        }
    }
}

// UI drawing functions remain the same...

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let api_port = config.network.api_port;
    let p2p_port = config.network.p2p_port;
    let db_path = config.database.path;

    // Set the API port as an environment variable for the API server to use
    env::set_var("PORT", api_port.to_string());

    let args: Vec<String> = env::args().collect();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let db = Database::open(&db_path).expect("Failed to open database");
    let blockchain = db.load_blockchain().unwrap_or_else(|_| {
        Blockchain::new([0; 32], 1).expect("Failed to create new blockchain")
    });

    // Create the unified Node
    let node = Arc::new(Node::new(blockchain));

    // Start API server in the background
    let api_node = node.clone();
    tokio::spawn(async move {
        if let Err(e) = trinitychain::api::run_api_server(api_node).await {
            eprintln!("API Server Error: {}", e);
        }
    });

    let stats = Arc::new(tokio::sync::Mutex::new(NodeStats {
        ..Default::default()
    }));

    let start_time = Instant::now();

    // Connect to peer if specified
    if args.len() >= 3 && args[1] == "--peer" {
        let peer_addr = args[2].clone();
        let node_clone = node.clone();
        tokio::spawn(async move {
            if let Some((host, port_str)) = peer_addr.split_once(':') {
                if let Ok(peer_port) = port_str.parse::<u16>() {
                    if let Err(e) = node_clone
                        .network
                        .clone()
                        .connect_peer(host.to_string(), peer_port)
                        .await
                    {
                        eprintln!("⚠️  Failed to connect to peer: {}", e);
                    }
                }
            }
        });
    }

    // Start P2P server
    let p2p_node = node.network.clone();
    tokio::spawn(async move {
        if let Err(e) = p2p_node.start_server(p2p_port).await {
            eprintln!("❌ Network error: {}", e);
        }
    });

    // UI and stats update loop
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        {
            let mut s = stats.lock().await;
            s.status = "Running".to_string();
            s.uptime_secs = start_time.elapsed().as_secs();

            let bc = node.blockchain.read().await;
            s.chain_height = bc.blocks.len() as u64;
            if let Some(last_block) = bc.blocks.last() {
                s.last_block_hash = hex::encode(last_block.hash());
            }

            let peers = node.network.list_peers().await;
            s.peer_count = peers.len();
            s.peers = peers.iter().map(|p| p.addr()).collect();
        }

        let _stats_clone = stats.lock().await.clone();
        terminal.draw(|_f| {
            // draw_ui(f, &stats_clone); // Assuming draw_ui is defined elsewhere
        })?;
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
