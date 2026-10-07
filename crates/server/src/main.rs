use tokio_util::sync::CancellationToken;

use crate::{
    game::{Game, GameSystem},
    network::Network,
};

mod game;
mod network;

#[tokio::main]
async fn main() {
    let shutdown = CancellationToken::new();
    let host_shutdown = shutdown.clone();
    tokio::task::spawn_blocking(|| {
        host(host_shutdown);
    });
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = shutdown.cancelled() => {},
    }
    println!("\rShutting down...");

    shutdown.cancel();
}
fn host(shutdown: CancellationToken) {
    let mut network = Network::new(shutdown.clone());
    println!("\r[Server] Initialized Network");
    let mut game = Game::new();
    GameSystem::run(&mut game, &mut network, shutdown);
}
