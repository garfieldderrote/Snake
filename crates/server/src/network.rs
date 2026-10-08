use futures_util::{SinkExt, StreamExt};
use snake_core::{
    Board, Deserialize, Difference, Dir, IVec2, InputPacket, OutputPacket, PlayerId, Serialize,
    Snake,
};
use std::collections::{HashMap, VecDeque};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;
use tracy_client::{set_thread_name, span};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
};

pub struct Network {
    receiver: Receiver<Packet>,
    senders: HashMap<PlayerId, Sender<Vec<u8>>>,
    buffer: HashMap<PlayerId, VecDeque<Dir>>,
    players: Vec<PlayerId>,
}

enum Packet {
    Direction {
        player: PlayerId,
        dir: Dir,
    },
    NewConnection {
        player: PlayerId,
        sender: mpsc::Sender<Vec<u8>>,
    },
    Disconnect {
        player: PlayerId,
    },
    Ping,
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
            set_thread_name!("Connection Listener");
            let weblistener = TcpListener::bind("0.0.0.0:9000").await.unwrap();
            let tcplistener = TcpListener::bind("0.0.0.0:9001").await.unwrap();
            println!("[Server/Network] Init Bind");
            let mut next_player_id = 0u64;
            loop {
                let websocket;
                let (socket, addr) = tokio::select! {
                    Ok(socket) = tcplistener.accept() => {websocket = false; socket},
                    Ok(socket) = weblistener.accept() => {websocket = true; socket},
                    _ = listener_shutdown.cancelled() => {
                        break;
                    }
                };
                println!("New Connection! from {} (websocket: {})", addr, websocket);

                let player_id = PlayerId(next_player_id);
                next_player_id += 1;

                let tx = tx.clone();
                //println!("{}", next_player_id);
                let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>(32);
                _ = tx
                    .send(Packet::NewConnection {
                        player: player_id,
                        sender: out_tx,
                    })
                    .await;
                let handler_shutdown = shutdown.clone();

                tokio::spawn(async move {
                    set_thread_name!("Player Listener/Sender");
                    if websocket {
                        handle_websocket(socket, player_id, tx, out_rx, handler_shutdown).await
                    } else {
                        handle_connection(socket, player_id, tx, out_rx, handler_shutdown).await
                    }
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
                Packet::Ping => {
                    //self.send_pong();
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
        let data = {
            let span = span!("Output Serialize");
            span.emit_color(0xFF0000);
            packet.serialize()
        };
        for tx in senders {
            let _ = tx.try_send(data.clone());
        }
    }
    pub fn add_snake(&self, snake: Snake) {
        let packet = OutputPacket::Difference(Difference::AddSnake(snake));
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
    // pub fn send_pong(&self) {
    //     let packet = OutputPacket::Pong;
    //     self.send_packet(packet);
    // }
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
    mut out_rx: mpsc::Receiver<Vec<u8>>,
    shutdown: CancellationToken,
) {
    let (mut reader, mut writer) = socket.into_split();
    //let mut lines = BufReader::new(reader).lines();
    // get the reason the loop exits
    let reason = loop {
        tokio::select! {
            result = reader.read_u64() => {
            match result {
                Ok(length) => {
                    //TODO: length bounds check
                    let mut buf = vec![0;length as usize];
                    reader.read_exact(&mut buf).await.unwrap();
                    if let Ok(InputPacket::Direction { dir }) = InputPacket::deserialize(&buf)
                    && tx.send(Packet::Direction {
                        player: player_id,
                        dir,
                    }).await.is_err()
                    {
                        break ConnectionEnd::Disconnect;
                    }
                }


                Err(_) => break ConnectionEnd:: Disconnect,
            }
        }

            Some(packet) = out_rx.recv() => {

                if writer.write_all(&packet.len().to_be_bytes()).await.is_err() {
                    break ConnectionEnd::Disconnect;
                }

                if writer.write_all(packet.as_slice()).await.is_err() {
                    break ConnectionEnd::Disconnect;
                }

                {
                    let span = span!("Finished Sending");
                    span.emit_color(0x0000FF);
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
async fn handle_websocket(
    socket: TcpStream,
    player_id: PlayerId,
    tx: mpsc::Sender<Packet>,
    mut out_rx: mpsc::Receiver<Vec<u8>>,
    shutdown: CancellationToken,
) {
    let mut websocket = match accept_async(socket).await {
        Ok(websocket) => websocket,
        Err(_) => return,
    };
    //let mut lines = BufReader::new(reader).lines();
    // get the reason the loop exits
    let reason = loop {
        tokio::select! {
            result = websocket.next() => {
            match result {
                Some(Ok(Message::Binary(buf))) => {
                    process_recieved_bytes(&buf, tx.clone(), player_id,&mut websocket).await;
                }
                Some(Ok(_)) => {}


                Some(Err(_)) => break ConnectionEnd:: Disconnect,
                None => break ConnectionEnd:: Disconnect,
            }
        }

            Some(packet) = out_rx.recv() => {

                if websocket.send(Message::Binary(packet.into())).await.is_err(){
                    break ConnectionEnd::Disconnect;
                }

                {
                    let span = span!("Finished Sending");
                    span.emit_color(0x0000FF);
                }

            }
                _ = shutdown.cancelled() => {
                    break ConnectionEnd::Shutdown;
                }

                else => break ConnectionEnd::Disconnect,
            }
    };

    if matches!(reason, ConnectionEnd::Disconnect) {
        _ = tx.send(Packet::Disconnect { player: player_id }).await;
    }
}

async fn process_recieved_bytes(
    bytes: &[u8],
    tx: Sender<Packet>,
    player_id: PlayerId,
    websocket: &mut WebSocketStream<TcpStream>,
) {
    match InputPacket::deserialize(bytes) {
        Ok(InputPacket::Direction { dir }) => {
            if tx
                .send(Packet::Direction {
                    player: player_id,
                    dir,
                })
                .await
                .is_err()
            {}
        }
        Ok(InputPacket::Ping) => {
            let packet = OutputPacket::Pong;
            let data = packet.serialize();

            _ = websocket.send(Message::Binary(data.into())).await.is_err();
        }
        Err(_) => {}
    }
}
