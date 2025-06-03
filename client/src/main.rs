use rand::{Rng, rng, seq::IndexedRandom};
use shared::TCP_PORT;

use crate::client::Client;
use chrono::Local;

mod client;
mod user_input_handler;

#[tokio::main]
async fn main() {
    let server_addr = format!("127.0.0.1:{}", TCP_PORT);
    let username = generate_username();

    let mut client = match Client::connect(&server_addr, &username).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error connecting: {}", e);
            return;
        }
    };

    print_connected_message(username, server_addr);

    if let Err(e) = client.run().await {
        eprintln!("Error: {}", e);
        return;
    }
}

fn clear_terminal() {
    print!("\x1B[2J\x1B[1;1H");
    use std::io::{Write, stdout};
    stdout().flush().unwrap();
}

fn print_connected_message(username: String, server_addr: String) {
    clear_terminal();

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

    println!("\nAvailable Commands:");
    println!("    - list users        : Show all connected users");
    println!("    - list rooms        : Show all available rooms");
    println!("    - connect <room>    : Connect to a specific room");
    println!("    - exit              : Quit the application");
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
