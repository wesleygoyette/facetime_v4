mod ascii_converter;
mod call_handler;
mod camera;
mod client;
mod user_input_handler;

use crate::{ascii_converter::AsciiConverter, camera::TestPatten, client::Client};
use chrono::Local;
use clap::Parser;
use rand::{Rng, rng, seq::IndexedRandom};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    username: Option<String>,

    #[arg(short, long, default_value = "137.66.0.54")]
    server_address: String,

    #[arg(short, long)]
    test_pattern: Option<TestPatten>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let username = match args.username {
        Some(username) => username,
        None => generate_username(),
    };

    let mut client = match Client::connect(&args.server_address, &username).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error connecting: {}", e);
            return;
        }
    };

    print_connected_message(username, args.server_address);

    if let Err(e) = client.run(args.test_pattern).await {
        eprintln!("Error: {}", e);
        return;
    }
}

fn print_connected_message(username: String, server_addr: String) {
    AsciiConverter::clear_terminal();

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let connection_status = "Connection OK";
    let info = [
        ("Time", timestamp.as_str()),
        ("Server", server_addr.as_str()),
        ("User", username.as_str()),
        ("Status", connection_status),
    ];

    let title = "╔══ Connected to WeSFU (version 4) ══╗";
    let total_width = title.len() - 12;
    println!("{}", title);

    for (label, value) in info {
        let content = format!("{}: {}", label, value);
        let padding = total_width.saturating_sub(content.chars().count() + 3);
        println!("║ {}{}║", content, " ".repeat(padding));
    }

    println!("╚{}╝", "═".repeat(total_width - 2));

    println!();

    print_available_commands();
}

fn print_available_commands() {
    println!("Available Commands:");
    println!("    - list users                  : Show all connected users");
    println!("    - list rooms                  : Show all available rooms");
    println!("    - create room <name>          : Create a new room");
    println!("    - join room <name>            : Connect to a specific room");
    println!("    - exit                        : Quit the application");
    println!("\nType a command to get started:\n");
}

fn generate_username() -> String {
    let adjectives = ["fast", "lazy", "cool", "smart", "brave"];
    let nouns = ["Tiger", "Eagle", "Lion", "Panda", "Wolf"];

    let mut rng = rng();

    let adjective = adjectives.choose(&mut rng).unwrap();
    let noun = nouns.choose(&mut rng).unwrap();
    let number: u16 = rng.random_range(1..9999);

    format!("{}{}{}", adjective, noun, number)
}
