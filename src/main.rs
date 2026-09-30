use std::io::{Write, stdin, stdout};

use tokio_util::sync::CancellationToken;

use crate::{
    client::input::Input,
    server::{
        game::{Game, GameSystem},
        network::Network,
    },
    util::GameEnding,
};

mod board;
mod client;
mod server;
mod snake;
mod util;

enum LaunchOptions {
    Join(String),
    Host,
    ServerOnly,
}

#[tokio::main]
async fn main() {
    let shutdown = CancellationToken::new();
    let choice = get_user_choice();
    match choice {
        LaunchOptions::Join(ip) => {
            join(ip, shutdown.clone()).await;
        }
        LaunchOptions::Host => {
            let host_shutdown = shutdown.clone();
            tokio::task::spawn_blocking(|| {
                host(host_shutdown);
            });
            join("localhost".to_string(), shutdown.clone()).await;
        }
        LaunchOptions::ServerOnly => {
            let host_shutdown = shutdown.clone();
            tokio::task::spawn_blocking(|| {
                host(host_shutdown);
            });
        }
    }
    // Waits for Ctrl_C or the token to cancel
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = shutdown.cancelled() => {},
    }
    println!("Shutting down...");

    shutdown.cancel();

    Input::clean();
}

async fn join(ip: String, shutdown: CancellationToken) {
    let net_shutdown = shutdown.clone();
    println!("\r[Client] Init Input");
    let input = Input::new(shutdown);
    println!("\r[Client] Init Network");
    let network = client::network::Network::new(ip).await;
    if let Some(network) = network {
        println!("\r[Client] Spawn Threads");
        network.spawn_threads(input, net_shutdown);
    } else {
        print!("\r[Client] Connection Failed");
    }
}

fn host(shutdown: CancellationToken) {
    let mut network = Network::new(shutdown.clone());
    println!("\r[Server] Initialized Network");
    let mut game = Game::new();
    let result = GameSystem::run(&mut game, &mut network, shutdown);
    match result {
        GameEnding::Victory => print_victory(),
        GameEnding::Failure => print_gameover(),
        GameEnding::Misc => {}
    }
}

fn get_user_choice() -> LaunchOptions {
    println!("1) Join a Game \n 2) Host a Game \n 3) Server Only \n Choose one of the Options:");
    loop {
        match read_string().as_str() {
            "1" => LaunchOptions::Join,
            "2" => return LaunchOptions::Host,
            "3" => return LaunchOptions::ServerOnly,
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

fn print_victory() {
    println!("You Win!");
}

fn print_gameover() {
    println!("Game Over!");
}
