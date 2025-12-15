//! Check wallet balance - Beautiful edition!

use colored::*;
use comfy_table::presets::UTF8_FULL;
use comfy_table::Color as TableColor;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use trinitychain::crypto::address_from_hex;
use trinitychain::persistence::Database;

const LOGO: &str = r#"
╔═══════════════════════════════════════════════════════════════╗
║      ████████╗██████╗ ██╗███╗   ██╗██╗████████╗██╗   ██╗     ║
║      ╚══██╔══╝██╔══██╗██║████╗  ██║██║╚══██╔══╝╚██╗ ██╔╝     ║
║         ██║   ██████╔╝██║██╔██╗ ██║██║   ██║    ╚████╔╝      ║
║         ██║   ██╔══██╗██║██║╚██╗██║██║   ██║     ╚██╔╝       ║
║         ██║   ██║  ██║██║██║ ╚████║██║   ██║      ██║        ║
║         ╚═╝   ╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝╚═╝   ╚═╝      ╚═╝        ║
║                  🔺 Wallet Balance 🔺                         ║
╚═══════════════════════════════════════════════════════════════╝
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", LOGO.bright_cyan());

    let args: Vec<String> = std::env::args().collect();
    let home = std::env::var("HOME")?;

    // Check if address was provided as argument
    let my_address = if args.len() > 1 {
        args[1].clone()
    } else {
        // Otherwise, load from wallet file (support WALLET_NAME env var)
        let wallet_name = std::env::var("WALLET_NAME").unwrap_or_else(|_| String::new());
        let wallet_file = if wallet_name.is_empty() {
            format!("{}/.trinitychain/wallet.json", home)
        } else {
            format!("{}/.trinitychain/wallet_{}.json", home, wallet_name)
        };

        let wallet_content = std::fs::read_to_string(&wallet_file).map_err(|e| {
            eprintln!("{}", "╔══════════════════════════════════════════╗".red());
            eprintln!(
                "{}",
                "║         ❌ Wallet Not Found!            ║".red().bold()
            );
            eprintln!("{}", "╚══════════════════════════════════════════╝".red());
            eprintln!();
            eprintln!(
                "{}",
                "💡 Run 'trinity-wallet-new <name>' to create a wallet".yellow()
            );
            format!("No wallet found at {}: {}", wallet_file, e)
        })?;

        let wallet_data: serde_json::Value = serde_json::from_str(&wallet_content)
            .map_err(|e| format!("Failed to parse wallet: {}", e))?;

        wallet_data["address"]
            .as_str()
            .ok_or("Wallet address not found in wallet file")?
            .to_string()
    };

    let my_address_bytes = address_from_hex(&my_address)?;

    let db =
        Database::open("trinitychain.db").map_err(|e| format!("Failed to open database: {}", e))?;
    let chain = db
        .load_blockchain()
        .map_err(|e| format!("Failed to load blockchain: {}", e))?;

    println!(
        "{}",
        "┌─────────────────────────────────────────────────────────────┐".bright_green()
    );
    println!(
        "{}",
        "│                    💰 WALLET BALANCE                        │"
            .bright_green()
            .bold()
    );
    println!(
        "{}",
        "└─────────────────────────────────────────────────────────────┘".bright_green()
    );
    println!();

    let addr_display = if my_address.len() > 50 {
        format!(
            "{}...{}",
            &my_address[..24],
            &my_address[my_address.len() - 24..]
        )
    } else {
        my_address.to_string()
    };

    println!("{}", format!("📍 Address: {}", addr_display).cyan());

    let height = chain.blocks.last().map(|b| b.header.height).unwrap_or(0);
    println!("{}", format!("📊 Chain Height: {}", height).bright_blue());
    println!(
        "{}",
        format!("⛓️  Network: {}", "Mainnet".bright_magenta()).bright_blue()
    );
    println!();

    let mut my_triangles = 0;
    let mut total_area = trinitychain::geometry::Coord::from_num(0);
    let mut triangle_list = Vec::new();

    // Filter triangles owned by this address
    for (hash, triangle) in &chain.state.utxo_set {
        if triangle.owner == my_address_bytes {
            my_triangles += 1;
            total_area += triangle.effective_value();
            let hash_hex = hex::encode(hash);
            triangle_list.push((hash_hex, triangle.effective_value()));
        }
    }

    if my_triangles == 0 {
        println!(
            "{}",
            "╔══════════════════════════════════════════════════════════╗".yellow()
        );
        println!(
            "{}",
            "║              📭 No Triangles Found                       ║".yellow()
        );
        println!(
            "{}",
            "╠══════════════════════════════════════════════════════════╣".yellow()
        );
        println!(
            "{}",
            "║  Your wallet is empty. Start mining or receive          ║".yellow()
        );
        println!(
            "{}",
            "║  triangles from another address to see your balance.    ║".yellow()
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════════╝".yellow()
        );
        println!();
        println!("{}", "💡 Start mining: miner <your_address>".bright_blue());
        return Ok(());
    }

    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════╗".green()
    );
    println!(
        "{}",
        format!(
            "║               🔺 Your Triangles ({})                     ║",
            my_triangles
        )
        .green()
        .bold()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════╝".green()
    );
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("#")
                .fg(TableColor::Cyan)
                .add_attribute(Attribute::Bold),
            Cell::new("Triangle Hash")
                .fg(TableColor::Cyan)
                .add_attribute(Attribute::Bold),
            Cell::new("Area")
                .fg(TableColor::Cyan)
                .add_attribute(Attribute::Bold),
        ]);

    triangle_list.sort_by(|a, b| b.1.cmp(&a.1));

    for (idx, (hash, area)) in triangle_list.iter().enumerate() {
        let hash_short = if hash.len() > 20 {
            format!("{}...{}", &hash[..10], &hash[hash.len() - 10..])
        } else {
            hash.clone()
        };

        table.add_row(vec![
            Cell::new(format!("{}", idx + 1)).fg(TableColor::Yellow),
            Cell::new(&hash_short).fg(TableColor::Green),
            Cell::new(format!("{:.6}", area)).fg(TableColor::Magenta),
        ]);
    }

    println!("{}", table);
    println!();

    println!(
        "{}",
        "╔══════════════════════════════════════════════════════════╗".bright_green()
    );
    println!(
        "{}",
        "║                    💎 TOTAL BALANCE                      ║"
            .bright_green()
            .bold()
    );
    println!(
        "{}",
        "╠══════════════════════════════════════════════════════════╣".bright_green()
    );
    println!(
        "{}",
        format!("║  🔺 Triangles: {:<42} ║", my_triangles).green()
    );
    println!(
        "{}",
        format!("║  📐 Total Area: {:<39.6} ║", total_area).green()
    );

    let avg_area = total_area / trinitychain::geometry::Coord::from_num(my_triangles);
    println!(
        "{}",
        format!("║  📊 Average Area: {:<37.6} ║", avg_area).green()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════╝".bright_green()
    );
    println!();

    println!("{}", "✨ Your wallet is looking good!".bright_blue());
    println!();

    Ok(())
}
