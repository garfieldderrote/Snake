use std::{thread::sleep, time::Duration};

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

#[tokio::main]
async fn main() {
    tokio::task::spawn_blocking(|| {
        host();
        loop {
            sleep(Duration::from_millis(100));
            println!("\rHello from blocking");
        }
    });

    join().await;
    std::future::pending::<()>().await;
}

async fn join() {
    println!("\r[Client] Init Input");
    let input = Input::new();
    println!("\r[Client] Init Network");
    let network = client::network::Network::new().await;
    if let Some(network) = network {
        println!("\r[Client] Spawn Threads");
        network.spawn_threads(input);
    } else {
        print!("\r[Client] Connection Failed");
    }
}

fn host() {
    let mut network = Network::new();
    println!("\r[Server] Initialized Network");
    let mut game = Game::new();
    let result = GameSystem::run(&mut game, &mut network);
    match result {
        GameEnding::Victory => print_victory(),
        GameEnding::Failure => print_gameover(),
        GameEnding::Misc => {}
    }
}

fn print_victory() {
    println!("You Win!");
}

fn print_gameover() {
    println!("Game Over!");
}
