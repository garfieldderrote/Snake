use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
};

use crate::{board::Board, server::game::PlayerId, snake::Snake, util::Dir};
pub struct Network {
    receiver: Receiver<Packet>,
    senders: HashMap<PlayerId, Sender<OutputPacket>>,
    buffer: HashMap<PlayerId, VecDeque<Dir>>,
    players: Vec<PlayerId>,
}
#[derive(Serialize, Deserialize)]
pub enum InputPacket {
    Direction { dir: Dir },
}
#[derive(Serialize, Deserialize, Clone)]
pub enum OutputPacket {
    State {
        board_state: Board,
        snakes: Vec<Snake>,
    },
}

enum Packet {
    Direction {
        player: PlayerId,
        dir: Dir,
    },
    NewConnection {
        player: PlayerId,
        sender: mpsc::Sender<OutputPacket>,
    },
}

impl Network {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<Packet>(100);
        let network = Network {
            receiver: rx,
            senders: HashMap::new(),
            buffer: HashMap::new(),
            players: Vec::new(),
        };

        tokio::spawn(async move {
            let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
            println!("[Server/Network] Init Bind");
            let mut next_player_id = 0u64;
            loop {
                let (socket, _) = listener.accept().await.unwrap();

                let player_id = PlayerId(next_player_id);
                next_player_id += 1;

                let tx = tx.clone();
                //println!("{}", next_player_id);
                let (out_tx, out_rx) = mpsc::channel::<OutputPacket>(32);
                _ = tx
                    .send(Packet::NewConnection {
                        player: player_id,
                        sender: out_tx,
                    })
                    .await;

                tokio::spawn(async move { handle_connection(socket, player_id, tx, out_rx).await });
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
            } else if let Packet::NewConnection { player, sender } = input {
                self.players.push(player);
                self.senders.insert(player, sender);
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
    pub fn send_game_state(&self, board: Board, snakes: Vec<Snake>) {
        let packet = OutputPacket::State {
            board_state: board,
            snakes,
        };
        let senders: Vec<_> = self.senders.values().cloned().collect();
        tokio::spawn(async move {
            for tx in senders {
                let _ = tx.send(packet.clone()).await;
            }
        });
    }
}

async fn handle_connection(
    socket: TcpStream,
    player_id: PlayerId,
    tx: mpsc::Sender<Packet>,
    mut out_rx: mpsc::Receiver<OutputPacket>,
) {
    let (reader, mut writer) = socket.into_split();
    let mut lines = BufReader::new(reader).lines();
    loop {
        tokio::select! {
            result = lines.next_line() => {
                match result {
                    Ok(Some(line)) => {
                        if let Ok(InputPacket::Direction { dir }) =
                            serde_json::from_str(&line)
                        {
                            if tx.send(Packet::Direction {
                                player: player_id,
                                dir,
                            }).await.is_err() {
                                break;
                            }
                        }
                    }

                    Ok(None) => break,

                    Err(_) => break,
                }
            }

            Some(packet) = out_rx.recv() => {
                let json = match serde_json::to_string(&packet) {
                    Ok(json) => json,
                    Err(_) => continue,
                };

                if writer.write_all(json.as_bytes()).await.is_err() {
                    break;
                }

                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
            }

            else => break,
        }
    }
}
