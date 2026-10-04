use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tokio_util::sync::CancellationToken;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
};

use crate::{
    board::Board,
    server::game::PlayerId,
    snake::Snake,
    util::{Dir, IVec2},
};
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
pub enum Difference {
    RemovedApples(Vec<IVec2>),
    AddedApples(Vec<IVec2>),
    SnakeMovements(HashMap<PlayerId, Dir>),
    AddSnake { player: PlayerId, snake: Snake },
    RemoveSnake(PlayerId),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum OutputPacket {
    State {
        board_state: Board,
        snakes: HashMap<PlayerId, Snake>,
    },
    Difference(Difference),
    TickFinished,
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
    Disconnect {
        player: PlayerId,
    },
}

impl Network {
    pub fn new(shutdown: CancellationToken) -> Self {
        let (tx, rx) = mpsc::channel::<Packet>(100);
        let network = Network {
            receiver: rx,
            senders: HashMap::new(),
            buffer: HashMap::new(),
            players: Vec::new(),
        };
        let listener_shutdown = shutdown.clone();
        tokio::spawn(async move {
            let listener = TcpListener::bind("0.0.0.0:9000").await.unwrap();
            println!("[Server/Network] Init Bind");
            let mut next_player_id = 0u64;
            loop {
                let (socket, _) = tokio::select! {
                    Ok(socket) = listener.accept() => socket,
                    _ = listener_shutdown.cancelled() => {
                        break;
                    }
                };

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
                let handler_shutdown = shutdown.clone();

                tokio::spawn(async move {
                    handle_connection(socket, player_id, tx, out_rx, handler_shutdown).await
                });
            }
        });
        network
    }
    pub fn receive(&mut self) -> (Vec<PlayerId>, Vec<PlayerId>) {
        let mut disconnected_players = Vec::new();
        let mut connected_players = Vec::new();
        while let Ok(input) = self.receiver.try_recv() {
            match input {
                Packet::Direction { player, dir } => {
                    if self.buffer.len() <= 2 {
                        self.buffer.entry(player).or_default();
                        self.buffer.get_mut(&player).unwrap().push_back(dir);
                    }
                }
                Packet::NewConnection { player, sender } => {
                    self.players.push(player);
                    self.senders.insert(player, sender);
                    connected_players.push(player);
                }
                Packet::Disconnect { player } => {
                    self.senders.remove(&player);
                    self.buffer.remove(&player);
                    self.players.retain(|p| p != &player);
                    disconnected_players.push(player);
                }
            };
        }
        (disconnected_players, connected_players)
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

    pub fn send_game_state(&self, board: Board, snakes: HashMap<PlayerId, Snake>) {
        let packet = OutputPacket::State {
            board_state: board,
            snakes,
        };
        self.send_packet(packet);
    }

    pub fn send_packet(&self, packet: OutputPacket) {
        let senders: Vec<_> = self.senders.values().cloned().collect();
        tokio::spawn(async move {
            for tx in senders {
                let _ = tx.send(packet.clone()).await;
            }
        });
    }
    pub fn add_snake(&self, player: PlayerId, snake: Snake) {
        let packet = OutputPacket::Difference(Difference::AddSnake { player, snake });
        self.send_packet(packet);
    }
    pub fn remove_snake(&self, player: PlayerId) {
        let packet = OutputPacket::Difference(Difference::RemoveSnake(player));
        self.send_packet(packet);
    }
    pub fn snake_movements(&self, movements: HashMap<PlayerId, Dir>) {
        let packet = OutputPacket::Difference(Difference::SnakeMovements(movements));
        self.send_packet(packet);
    }
    pub fn remove_apples(&self, removed_apples: Vec<IVec2>) {
        let packet = OutputPacket::Difference(Difference::RemovedApples(removed_apples));
        self.send_packet(packet);
    }
    pub fn add_apples(&self, added_apples: Vec<IVec2>) {
        let packet = OutputPacket::Difference(Difference::AddedApples(added_apples));
        self.send_packet(packet);
    }
    pub fn send_finished(&self) {
        let packet = OutputPacket::TickFinished;
        self.send_packet(packet);
    }
}

enum ConnectionEnd {
    Shutdown,
    Disconnect,
}

// handles the whole packet receiving and sending over the network through channels
// on tx it sends the incoming packets
// on out_rx it receives packets to be send
async fn handle_connection(
    socket: TcpStream,
    player_id: PlayerId,
    tx: mpsc::Sender<Packet>,
    mut out_rx: mpsc::Receiver<OutputPacket>,
    shutdown: CancellationToken,
) {
    let (reader, mut writer) = socket.into_split();
    let mut lines = BufReader::new(reader).lines();
    // get the reason the loop exits
    let reason = loop {
        tokio::select! {
            result = lines.next_line() => {
                match result {
                Ok(Some(line)) => {
                    if let Ok(InputPacket::Direction { dir }) = serde_json::from_str(&line)
                    && tx.send(Packet::Direction {
                        player: player_id,
                        dir,
                    }).await.is_err()
                    {
                        break ConnectionEnd::Disconnect;
                    }
                }

                    Ok(None) => break ConnectionEnd::Disconnect,

                    Err(_) => break ConnectionEnd:: Disconnect,
                }
            }

            Some(packet) = out_rx.recv() => {
                let json = match serde_json::to_string(&packet) {
                    Ok(json) => json,
                    Err(_) => continue,
                };

                if writer.write_all(json.as_bytes()).await.is_err() {
                    break ConnectionEnd::Disconnect;
                }

                if writer.write_all(b"\n").await.is_err() {
                    break ConnectionEnd::Disconnect;
                }
            }
            _ = shutdown.cancelled() => {
                break ConnectionEnd::Shutdown;
            }

            else => break ConnectionEnd::Disconnect,
        }
    };
    if matches!(reason, ConnectionEnd::Disconnect) {
        _ = writer.shutdown().await;
        _ = tx.send(Packet::Disconnect { player: player_id }).await;
    }
}
