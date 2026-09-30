use std::time::Duration;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    time::sleep,
};

use crate::{
    client::{
        draw::{self, ClientWorld, draw_board},
        input::Input,
    },
    server::network::{InputPacket, OutputPacket},
};

pub struct Network {
    stream: TcpStream,
}

impl Network {
    pub async fn new() -> Option<Self> {
        sleep(Duration::from_millis(10)).await;
        println!("\r[Client/Network] new()");
        if let Ok(stream) = TcpStream::connect("127.0.0.1:9000").await {
            println!("\r[Client/Network] Init");
            return Some(Network { stream });
        }
        println!("\r[Client/Network] Init Failed");
        None
    }
    pub fn spawn_threads(self, input: Input) {
        let (reader, writer) = self.stream.into_split();
        Network::spawn_listen_thread(reader);
        Network::spawn_send_thread(writer, input);
    }
    pub fn spawn_listen_thread(reader: OwnedReadHalf) {
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(packet) = serde_json::from_str::<OutputPacket>(&line) {
                    if let OutputPacket::State {
                        board_state,
                        snakes,
                    } = packet
                    {
                        let mut world = ClientWorld::new(board_state, snakes);
                        draw_board(&mut world);
                    }
                }
            }
        });
    }
    pub fn spawn_send_thread(mut writer: OwnedWriteHalf, mut input: Input) {
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(50));
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
