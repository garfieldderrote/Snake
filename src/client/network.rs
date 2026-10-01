use std::time::Duration;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    time::sleep,
};
use tokio_util::sync::CancellationToken;

use crate::{
    client::{
        draw::{ClientWorld, draw_board},
        input::Input,
    },
    server::network::{InputPacket, OutputPacket, ServerState},
};

pub struct Network {
    stream: TcpStream,
}

impl Network {
    pub async fn new(ip: String) -> Option<Self> {
        sleep(Duration::from_millis(10)).await;
        println!("\r[Client/Network] new()");
        if let Ok(stream) = TcpStream::connect(format!("{}:9000", ip)).await {
            println!("\r[Client/Network] Init");
            return Some(Network { stream });
        }
        println!("\r[Client/Network] Init Failed");
        None
    }
    pub fn spawn_threads(self, input: Input, shutdown: CancellationToken) {
        let (reader, writer) = self.stream.into_split();
        Network::spawn_listen_thread(reader, shutdown.clone());
        Network::spawn_send_thread(writer, input, shutdown.clone());
    }
    pub fn spawn_listen_thread(reader: OwnedReadHalf, shutdown: CancellationToken) {
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = tokio::select! {
                line = lines.next_line() => line,
                _ = shutdown.cancelled() => Ok(None),
            } {
                if let Ok(packet) = serde_json::from_str::<OutputPacket>(&line) {
                    Network::handle_received_packet(packet);
                }
            }
            println!("Disconnected from Server... ");
            shutdown.cancel();
        });
    }
    fn handle_received_packet(packet: OutputPacket) {
        match packet {
            OutputPacket::State {
                board_state,
                snakes,
            } => {
                let mut world = ClientWorld::new(board_state, snakes);
                draw_board(&mut world);
            }
            OutputPacket::ServerState(ServerState::Shutdown) => {}
            OutputPacket::ServerState(_) => {}
        }
    }
    pub fn spawn_send_thread(
        mut writer: OwnedWriteHalf,
        mut input: Input,
        shutdown: CancellationToken,
    ) {
        tokio::spawn(async move {
            while !shutdown.is_cancelled() {
                sleep(Duration::from_millis(50)).await;
                input.fetch();
                if let Some(dir) = input.get_dir() {
                    let packet = serde_json::to_string(&InputPacket::Direction { dir });
                    if packet.is_err() {
                        continue;
                    }
                    let packet = packet.unwrap();
                    _ = writer.write_all(packet.as_bytes()).await;
                    _ = writer.write_all(b"\n").await;
                }
            }
        });
    }
}
