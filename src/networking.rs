use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
};

use crate::{game::PlayerId, util::Dir};
pub struct Network {
    receiver: Receiver<Packet>,
    buffer: HashMap<PlayerId, VecDeque<Dir>>,
    players: Vec<PlayerId>,
}
#[derive(Serialize, Deserialize)]
pub enum InputPacket {
    Direction { dir: Dir },
}

enum Packet {
    Direction { player: PlayerId, dir: Dir },
    NewConnection(PlayerId),
}

impl Network {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<Packet>(100);
        let network = Network {
            receiver: rx,
            buffer: HashMap::new(),
            players: Vec::new(),
        };

        tokio::spawn(async move {
            let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
            let mut next_player_id = 0u64;
            loop {
                let (socket, _) = listener.accept().await.unwrap();

                let player_id = PlayerId(next_player_id);
                next_player_id += 1;

                let tx = tx.clone();
                //println!("{}", next_player_id);

                _ = tx.send(Packet::NewConnection(player_id)).await;

                tokio::spawn(async move { handle_connection(socket, player_id, tx).await });
            }
        });
        network
    }
    pub fn receive(&mut self) {
        while let Ok(input) = self.receiver.try_recv() {
            if self.buffer.len() <= 2
                && let Packet::Direction { player, dir } = input
            {
                if !self.buffer.contains_key(&player) {
                    self.buffer.insert(player, VecDeque::new());
                }
                self.buffer.get_mut(&player).unwrap().push_back(dir);
            } else if let Packet::NewConnection(player) = input {
                self.players.push(player);
            }
        }
    }
    pub fn get_input_player(&mut self, player_id: PlayerId) -> Option<Dir> {
        if let Some(buffer) = self.buffer.get_mut(&player_id) {
            let dir = buffer.pop_front();
            return dir;
        }
        None
    }
    pub fn get_players(&self) -> &Vec<PlayerId> {
        &self.players
    }
}

async fn handle_connection(socket: TcpStream, player_id: PlayerId, tx: mpsc::Sender<Packet>) {
    let reader = BufReader::new(socket);
    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let input: Result<InputPacket, _> = serde_json::from_str(&line);

        if let Ok(InputPacket::Direction { dir: direction }) = input
            && tx
                .send(Packet::Direction {
                    player: player_id,
                    dir: direction,
                })
                .await
                .is_err()
        {
            break;
        }
    }
}
