use std::io::{Write, stdin, stdout};

use tokio_util::sync::CancellationToken;
use tracy_client::Client;

use crate::{input::Input, network::Network};

mod draw;
mod input;
mod network;

enum LaunchOptions {
    Join(String),
    Host,
    ServerOnly,
    Quit,
}

#[tokio::main]
async fn main() {
    let shutdown = CancellationToken::new();
    let client_shutdown = shutdown.clone();
    let _client = Client::start(); // for tracy debug
    join("localhost".to_string(), client_shutdown).await;
    // Waits for Ctrl_C or the token to cancel
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = shutdown.cancelled() => {},
    }
    Input::clean();
    println!("\rShutting down...");

    shutdown.cancel();
}

pub async fn join(ip: String, shutdown: CancellationToken) {
    let net_shutdown = shutdown.clone();
    println!("\r[Client] Init Input");
    let input = Input::new(shutdown.clone());
    println!("\r[Client] Init Network");
    let network = Network::new(ip).await;
    if let Some(network) = network {
        println!("\r[Client] Spawn Threads");
        network.spawn_threads(input, net_shutdown);
    } else {
        println!("\r[Client] Connection Failed");
        shutdown.cancel();
    }
}

fn get_user_choice() -> LaunchOptions {
    println!(
        " 1) Join a Game \n 2) Host a Game \n 3) Server Only \n 4) Quit \n Choose one of the Options:"
    );
    loop {
        match read_string().as_str() {
            "1" => LaunchOptions::Join,
            "2" => return LaunchOptions::Host,
            "3" => return LaunchOptions::ServerOnly,
            "4" => return LaunchOptions::Quit,
            _ => {
                continue;
            }
        };
        println!("Enter IP Address");
        let ip = read_string();
        return LaunchOptions::Join(ip);
    }
}

fn read_string() -> String {
    let mut s = String::new();
    let _ = stdout().flush();
    stdin()
        .read_line(&mut s)
        .expect("Did not enter a correct string");
    if let Some('\n') = s.chars().next_back() {
        s.pop();
    }
    if let Some('\r') = s.chars().next_back() {
        s.pop();
    }
    s
}
