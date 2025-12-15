//! Send triangles to another address - Beautiful edition!

use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use trinitychain::cli::load_blockchain_from_config;
use trinitychain::crypto::address_from_hex;
use trinitychain::geometry::Coord;
use trinitychain::network::NetworkNode;
use trinitychain::transaction::{Transaction, TransferTx};
use trinitychain::wallet;

const LOGO: &str = r#"
╔═══════════════════════════════════════════════════════════════╗
║      ████████╗██████╗ ██╗███╗   ██╗██╗████████╗██╗   ██╗     ║
║      ╚══██╔══╝██╔══██╗██║████╗  ██║██║╚══██╔══╝╚██╗ ██╔╝     ║
║         ██║   ██████╔╝██║██╔██╗ ██║██║   ██║    ╚████╔╝      ║
║         ██║   ██╔══██╗██║██║╚██╗██║██║   ██║     ╚██╔╝       ║
║         ██║   ██║  ██║██║██║ ╚████║██║   ██║      ██║        ║
║         ╚═╝   ╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚═╝   ╚═╝      ╚═╝        ║
║                 🔺 Blockchain Transfer 🔺                     ║
╚═══════════════════════════════════════════════════════════════╝
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("{}", LOGO.bright_cyan());
        println!(
            "{}",
            "╔══════════════════════════════════════════════════════════╗".bright_yellow()
        );
        println!(
            "{}",
            "║                      📖 Usage Guide                      ║"
                .bright_yellow()
                .bold()
        );
        println!(
            "{}",
            "╠══════════════════════════════════════════════════════════╣".bright_yellow()
        );
        println!(
            "{}",
            "║                                                          ║".bright_yellow()
        );
        println!(
            "{}",
            "║  Usage:                                                  ║".bright_yellow()
        );
        println!(
            "{}",
            "║    send <to_address> <amount> [--from <wallet_name>] [memo] ║".white()
        );
        println!(
            "{}",
            "║                                                          ║".bright_yellow()
        );
        println!(
            "{}",
            "║  Examples:                                               ║".bright_yellow()
        );
        println!(
            "{}",
            "║    send abc123... 100                                    ║".white()
        );
        println!(
            "{}",
            "║    send abc123... 100 --from alice \"Payment for services\" ║".white()
        );
        println!(
            "{}",
            "║                                                          ║".bright_yellow()
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════════╝".bright_yellow()
        );
        println!();
        std::process::exit(1);
    }

    println!("{}", LOGO.bright_cyan());

    let to_address = &args[1];
    let to_address_bytes = address_from_hex(to_address)?;
    let amount: f64 = args[2].parse()?;
    let amount_coord = Coord::from_num(amount);

    let mut wallet_name: Option<String> = None;
    let mut memo: Option<String> = None;

    let mut i = 3;
    while i < args.len() {
        if args[i] == "--from" {
            wallet_name = Some(args[i + 1].clone());
            i += 2;
        } else {
            memo = Some(args[i..].join(" "));
            break;
        }
    }

    println!(
        "{}",
        "┌─────────────────────────────────────────────────────────────┐".bright_magenta()
    );
    println!(
        "{}",
        "│                  💸 INITIATING TRANSFER                     │"
            .bright_magenta()
            .bold()
    );
    println!(
        "{}",
        "└─────────────────────────────────────────────────────────────┘".bright_magenta()
    );
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );

    pb.set_message("Loading wallet...");
    pb.enable_steady_tick(Duration::from_millis(100));

    let from_wallet = if let Some(name) = wallet_name {
        wallet::load_named_wallet(&name)?
    } else {
        wallet::load_default_wallet()?
    };

    let from_address = from_wallet.address.clone();
    let from_address_bytes = address_from_hex(&from_address)?;
    let keypair = from_wallet.get_keypair()?;

    pb.set_message("Loading blockchain...");

    let (_config, mut chain) = load_blockchain_from_config()?;

    // Track locked triangles from pending transactions
    let mut locked_triangles = HashSet::new();

    // Load existing mempool from disk
    if let Ok(mempool_data) = std::fs::read_to_string("mempool.json") {
        let transactions: Result<Vec<Transaction>, _> = serde_json::from_str(&mempool_data);
        if let Ok(txs) = transactions {
            let txs_clone = txs.clone();

            for tx in txs {
                let _ = chain.mempool.add_transaction(tx);
            }

            if !chain.mempool.is_empty() {
                pb.println(format!(
                    "📬 {} pending transaction(s) already in mempool",
                    chain.mempool.len()
                ));
            }

            // Collect locked UTXOs from pending transfers
            for tx in txs_clone {
                if let Transaction::Transfer(transfer_tx) = tx {
                    locked_triangles.insert(transfer_tx.input_hash);
                }
            }
        }
    }
    pb.set_message("Finding a suitable triangle...");

    let (input_hash, _input_triangle) = chain
        .state
        .utxo_set
        .iter()
        .find(|(hash, triangle)| {
            triangle.owner == from_address_bytes
                && triangle.effective_value() >= amount_coord
                && !locked_triangles.contains(*hash)
        })
        .ok_or("No single triangle with sufficient value found for the transfer")?;

    pb.finish_and_clear();

    let from_display = if from_address.len() > 20 {
        format!(
            "{}...{}",
            &from_address[..10],
            &from_address[from_address.len() - 10..]
        )
    } else {
        from_address.clone()
    };
    let to_display = if to_address.len() > 20 {
        format!(
            "{}...{}",
            &to_address[..10],
            &to_address[to_address.len() - 10..]
        )
    } else {
        to_address.to_string()
    };

    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        "║              🔍 TRANSACTION DETAILS                      ║"
            .bright_cyan()
            .bold()
    );
    println!(
        "{}",
        "╠══════════════════════════════════════════════════════════╣".bright_cyan()
    );
    println!("{}", format!("║  👤 From: {:<47} ║", from_display).cyan());
    println!("{}", format!("║  🎯 To: {:<49} ║", to_display).cyan());
    println!("{}", format!("║  💸 Amount: {:<45} ║", amount).cyan());
    if let Some(ref m) = memo {
        let memo_display = if m.len() > 45 {
            format!("{}...", &m[..42])
        } else {
            m.clone()
        };
        println!("{}", format!("║  📝 Memo: {:<47} ║", memo_display).cyan());
    }
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════╝".bright_cyan()
    );
    println!();

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    pb.set_message("Creating transaction...");

    let fee = Coord::from_num(0);
    let mut tx = TransferTx::new(
        *input_hash,
        to_address_bytes,
        from_address_bytes,
        amount_coord,
        fee,
        chain.blocks.len() as u64,
    );

    if let Some(m) = memo {
        tx = tx.with_memo(m)?;
    }

    pb.set_message("Signing transaction...");

    let message = tx.signable_message();
    let signature = keypair.sign(&message)?;
    let public_key = keypair.public_key.serialize().to_vec();
    tx.sign(signature.to_vec(), public_key.to_vec());

    let transaction = Transaction::Transfer(tx);
    chain.mempool.add_transaction(transaction.clone())?;

    pb.set_message("Saving mempool...");
    let all_txs = chain.mempool.get_all_transactions();
    std::fs::write("mempool.json", serde_json::to_string(&all_txs)?)?;

    pb.set_message("Broadcasting to network...");

    let network_node = NetworkNode::new(Arc::new(RwLock::new(chain)));
    network_node.broadcast_transaction(&transaction).await;

    pb.finish_and_clear();

    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════╗".bright_green()
    );
    println!(
        "{}",
        "║              ✅ TRANSACTION SUCCESSFUL!                  ║"
            .bright_green()
            .bold()
    );
    println!(
        "{}",
        "╠══════════════════════════════════════════════════════════╣".bright_green()
    );
    println!(
        "{}",
        "║  Your transaction has been broadcasted to the network   ║".green()
    );
    println!(
        "{}",
        "║  and will be included in the next block!                ║".green()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════╝".bright_green()
    );
    println!();
    println!(
        "{}",
        "🎉 Transfer complete! The triangle is on its way!".bright_blue()
    );
    println!();

    Ok(())
}
