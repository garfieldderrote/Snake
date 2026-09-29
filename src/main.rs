use std::{thread::sleep, time::Duration};

use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::{
    game::{Game, GameSystem},
    input::Input,
    networking::Network,
    util::GameEnding,
};

mod board;
mod draw;
mod game;
mod input;
mod networking;
mod snake;
mod util;

#[tokio::main]
async fn main() {
    tokio::spawn(async move {
        sleep(Duration::from_millis(100));
        if let Ok(mut stream) = TcpStream::connect("127.0.0.1:9000").await {
            sleep(Duration::from_millis(1000));
            _ = stream.write_all(b"up\n").await;
            _ = stream.write_all(b"left\n").await;
            _ = stream.write_all(b"down\n").await;
        }
    });
    tokio::spawn(async move {
        sleep(Duration::from_millis(2000));
        if let Ok(mut stream) = TcpStream::connect("127.0.0.1:9000").await {
            sleep(Duration::from_millis(1000));
            _ = stream.write_all(b"down\n").await;
            _ = stream.write_all(b"right\n").await;
            _ = stream.write_all(b"up\n").await;
        }
    });
    let mut network = Network::new();
    let mut game = Game::new();
    let mut input = Input::new();
    let result = GameSystem::run(&mut game, &mut input, &mut network);
    Input::clean();
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
