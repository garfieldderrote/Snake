use std::{collections::HashMap, time::Duration};

use snake_core::{
    Board, DecodeError, Deserialize, Difference, InputPacket, OutputPacket, Serialize,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use tracy_client::span;

use crate::{
    draw::{ClientWorld, draw_board},
    input::Input,
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
    pub fn spawn_listen_thread(mut reader: OwnedReadHalf, shutdown: CancellationToken) {
        tokio::spawn(async move {
            let mut world = ClientWorld::new(Board::new(0, 0, 0), HashMap::new());
            while let Some(data) = tokio::select! {
                length = reader.read_u64() => {
                    let mut buf = vec![0; length.unwrap() as usize];
                    _ = reader.read_exact(&mut buf).await;
                    Some(buf)
                },
                _ = shutdown.cancelled() => None,
            } {
                let span = span!("Handle recieved Packet");
                span.emit_color(0x00FF00);
                match OutputPacket::deserialize(&data) {
                    Ok(packet) => {
                        Network::handle_received_packet(packet, &mut world);
                    }
                    Err(error) => match error {
                        DecodeError::InvalidPacketType(i) => {
                            println!("Failed on Packet number: {}", i);
                        }
                        _ => {
                            println!("Failed Packet read, {:?}", error);
                        }
                    },
                }
            }
            println!("Disconnected from Server... ");
            shutdown.cancel();
        });
    }
    fn handle_received_packet(packet: OutputPacket, world: &mut ClientWorld) {
        match packet {
            OutputPacket::State {
                board_state,
                snakes,
            } => {
                let span = span!("State");
                span.emit_color(0xFF0000);
                *world = ClientWorld::new(board_state, snakes);
            }
            OutputPacket::Difference(Difference::RemovedApples(apples)) => {
                let span = span!("RemoveApples");
                span.emit_color(0xFF0000);
                world.remove_apple(apples);
            }
            OutputPacket::Difference(Difference::AddedApples(apples)) => {
                let span = span!("AddApples");
                span.emit_color(0xFF0000);
                world.add_apples(apples);
            }
            OutputPacket::Difference(Difference::SnakeMovements(movements)) => {
                let span = span!("SnakeMovements");
                span.emit_color(0xFF0000);
                for (player, dir) in movements {
                    world.move_snake(player, dir);
                }
            }
            OutputPacket::Difference(Difference::AddSnake(snake)) => {
                let span = span!("AddSnake");
                span.emit_color(0xFF0000);
                world.add_snake(snake);
            }
            OutputPacket::Difference(Difference::RemoveSnake(player)) => {
                let span = span!("RemoveSnake");
                span.emit_color(0xFF0000);
                world.remove_snake(player);
            }
            OutputPacket::TickFinished => {
                let span = span!("Draw");
                span.emit_color(0xFF0000);
                draw_board(world);
            }
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
                    let packet = InputPacket::Direction { dir }.serialize();
                    _ = writer.write_all(&packet.len().to_be_bytes()).await;
                    _ = writer.write_all(packet.as_slice()).await;
                }
            }
        });
    }
}
