use std::collections::HashMap;

use crate::{
    PlayerId,
    board::Board,
    snake::{Segment, Snake, SnakeState::Alive},
    util::{Dir, IVec2},
};

pub enum InputPacket {
    Direction { dir: Dir },
    Handshake(String),
    Ping,
}

#[derive(Clone)]
pub enum Difference {
    RemovedApples(Vec<IVec2>),
    AddedApples(Vec<IVec2>),
    SnakeMovement(PlayerId, Dir, u64),
    AddSnake(Snake),
    RemoveSnake(PlayerId),
}

#[derive(Clone)]
pub enum OutputPacket {
    State {
        board_state: Board,
        snakes: HashMap<PlayerId, Snake>,
    },
    Difference(Difference),
    TickFinished(u64),
    Handshake(PlayerId, Vec<(PlayerId, String)>),
    Points(Vec<(PlayerId, u64)>),
    Pong,
}

pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}
pub trait Deserialize {
    fn deserialize(input: &[u8]) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

impl Serialize for OutputPacket {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        match self {
            OutputPacket::Difference(diff) => match diff {
                Difference::AddedApples(apples) => {
                    result.push(0x01);
                    for apple in apples {
                        result.extend_from_slice(&apple.x.to_be_bytes());
                        result.extend_from_slice(&apple.y.to_be_bytes());
                    }
                }
                Difference::RemovedApples(apples) => {
                    result.push(0x02);
                    for apple in apples {
                        result.extend_from_slice(&apple.x.to_be_bytes());
                        result.extend_from_slice(&apple.y.to_be_bytes());
                    }
                }
                Difference::AddSnake(snake) => {
                    result.push(0x03);
                    let snake = snake.serialize();
                    result.extend_from_slice(&(snake.len() as u16).to_be_bytes());
                    result.extend_from_slice(snake.iter().as_slice());
                }
                Difference::RemoveSnake(player) => {
                    result.push(0x04);
                    result.extend_from_slice(&player.0.to_be_bytes());
                }
                Difference::SnakeMovement(player, dir, timestamp) => {
                    result.push(0x05);
                    result.extend_from_slice(&player.0.to_be_bytes());
                    match dir {
                        Dir::Up => {
                            result.push(0x00);
                        }
                        Dir::Down => {
                            result.push(0x01);
                        }
                        Dir::Left => {
                            result.push(0x02);
                        }
                        Dir::Right => {
                            result.push(0x03);
                        }
                    }
                    result.extend_from_slice(&timestamp.to_be_bytes());
                }
            },
            OutputPacket::State {
                board_state,
                snakes,
            } => {
                result.push(0x00);
                result.extend_from_slice(&(board_state.get_apples().len() as u16).to_be_bytes());
                for apple in board_state.get_apples() {
                    result.extend_from_slice(&apple.x.to_be_bytes());
                    result.extend_from_slice(&apple.y.to_be_bytes());
                }
                result.extend_from_slice(&board_state.get_width().to_be_bytes());
                result.extend_from_slice(&board_state.get_height().to_be_bytes());
                for snake in snakes.values() {
                    let snake = snake.serialize();
                    result.extend_from_slice(&(snake.len() as u16).to_be_bytes());
                    result.extend_from_slice(snake.iter().as_slice());
                }
            }
            OutputPacket::TickFinished(number) => {
                result.push(0x06);
                result.extend_from_slice(&number.to_be_bytes());
            }
            OutputPacket::Pong => {
                result.push(0x07);
            }
            OutputPacket::Handshake(playerid, playernames) => {
                result.push(0x08);
                result.extend_from_slice(&playerid.0.to_be_bytes());
                for (id, name) in playernames {
                    result.extend_from_slice(&id.0.to_be_bytes());
                    result.extend_from_slice(&name.len().to_be_bytes());
                    result.extend_from_slice(&name.clone().into_bytes());
                }
            }
            OutputPacket::Points(player_points) => {
                result.push(0x09);
                for (id, points) in player_points {
                    result.extend_from_slice(&id.0.to_be_bytes());
                    result.extend_from_slice(&points.to_be_bytes());
                }
            }
        };
        result
    }
}
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

#[derive(Debug)]
pub enum DecodeError {
    UnexpectedEof,
    InvalidPacketType(u8),
    //InvalidPayloadLength,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        let byte = *self.buf.get(self.pos).ok_or(DecodeError::UnexpectedEof)?;

        self.pos += 1;
        Ok(byte)
    }

    fn u16_be(&mut self) -> Result<u16, DecodeError> {
        let bytes = self.read(2)?;

        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
    // fn u32_be(&mut self) -> Result<u32, DecodeError> {
    //     let bytes = self.read(4)?;
    //
    //     Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    // }
    fn i32_be(&mut self) -> Result<i32, DecodeError> {
        let bytes = self.read(4)?;

        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
    fn u64_be(&mut self) -> Result<u64, DecodeError> {
        let bytes = self.read(8)?;

        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or(DecodeError::UnexpectedEof)?;

        let bytes = self
            .buf
            .get(self.pos..end)
            .ok_or(DecodeError::UnexpectedEof)?;

        self.pos = end;
        Ok(bytes)
    }

    // fn remaining(&self) -> usize {
    //     self.buf.len() - self.pos
    // }
}

impl Deserialize for OutputPacket {
    fn deserialize(input: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(input);
        let packet_type = r.u8()?;
        let packet = match packet_type {
            0x00 => {
                let amount_apples = r.u16_be()?;
                let mut apples = Vec::new();
                for _ in 0..amount_apples {
                    let x = r.i32_be()?;
                    let y = r.i32_be()?;
                    apples.push(IVec2::new(x, y));
                }
                let width = r.i32_be()?;
                let height = r.i32_be()?;
                let board = Board {
                    size: (width, height),
                    num_apples: apples.len() as i32,
                    apples,
                };
                let mut snakes = HashMap::new();
                while let Ok(len) = r.u16_be() {
                    let snake = r.read(len as usize)?;
                    let snake = Snake::deserialize(snake)?;
                    snakes.insert(snake.player, snake);
                }
                OutputPacket::State {
                    board_state: board,
                    snakes,
                }
            }
            0x01 => {
                let mut apples = Vec::new();
                while let Ok(x) = r.i32_be() {
                    let y = r.i32_be()?;
                    apples.push(IVec2::new(x, y));
                }
                OutputPacket::Difference(Difference::AddedApples(apples))
            }
            0x02 => {
                let mut apples = Vec::new();
                while let Ok(x) = r.i32_be() {
                    let y = r.i32_be()?;
                    apples.push(IVec2::new(x, y));
                }
                OutputPacket::Difference(Difference::RemovedApples(apples))
            }
            0x03 => {
                // AddSnake
                let len = r.u16_be()?;
                let snake = r.read(len as usize)?;
                let snake = Snake::deserialize(snake)?;
                OutputPacket::Difference(Difference::AddSnake(snake))
            }
            0x04 => {
                let player = PlayerId(r.u64_be()?);
                OutputPacket::Difference(Difference::RemoveSnake(player))
            }
            0x05 => {
                let player = r.u64_be()?;
                let player = PlayerId(player);
                let dir = match r.u8()? {
                    0x00 => Dir::Up,
                    0x01 => Dir::Down,
                    0x02 => Dir::Left,
                    0x03 => Dir::Right,
                    _ => return Err(DecodeError::InvalidPacketType(0x05)),
                };
                let timestamp = r.u64_be()?;

                OutputPacket::Difference(Difference::SnakeMovement(player, dir, timestamp))
            }
            0x06 => {
                let number = r.u64_be()?;
                OutputPacket::TickFinished(number)
            }
            0x07 => OutputPacket::Pong,
            0x08 => {
                let playerid = PlayerId(r.u64_be()?);
                let mut playernames = Vec::new();
                while let Ok(player) = r.u64_be() {
                    let string_length = r.u64_be()? as usize;
                    let string_bytes = r.read(string_length)?;
                    let name = match String::from_utf8(string_bytes.to_vec()) {
                        Ok(string) => string,
                        Err(_) => return Err(DecodeError::InvalidPacketType(0x08)),
                    };
                    playernames.push((PlayerId(player), name));
                }
                OutputPacket::Handshake(playerid, playernames)
            }
            0x09 => {
                let mut player_points = Vec::new();
                while let Ok(player) = r.u64_be() {
                    let points = r.u64_be()?;
                    player_points.push((PlayerId(player), points));
                }
                OutputPacket::Points(player_points)
            }
            _ => {
                return Err(DecodeError::InvalidPacketType(0x06));
            }
        };
        Ok(packet)
    }
}

impl Serialize for Snake {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.player.0.to_be_bytes());
        //result.extend_from_slice(&(self.segments.len()).to_be_bytes());
        for segment in &self.segments {
            match segment.dir {
                Dir::Up => {
                    result.push(0x00);
                }
                Dir::Down => {
                    result.push(0x01);
                }
                Dir::Left => {
                    result.push(0x02);
                }
                Dir::Right => {
                    result.push(0x03);
                }
            }
            result.extend_from_slice(&segment.pos.x.to_be_bytes());
            result.extend_from_slice(&segment.pos.y.to_be_bytes());
        }
        result
    }
}

impl Deserialize for Snake {
    fn deserialize(input: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(input);
        let player = PlayerId(r.u64_be()?);
        let mut segments = Vec::new();
        let mut last_tail_dir = Dir::Right;
        while let Ok(dir) = r.u8() {
            let dir = match dir {
                0x00 => Dir::Up,
                0x01 => Dir::Down,
                0x02 => Dir::Left,
                0x03 => Dir::Right,
                _ => return Err(DecodeError::InvalidPacketType(0)),
            };
            last_tail_dir = dir;
            let x = r.i32_be()?;
            let y = r.i32_be()?;
            segments.push(Segment {
                dir,
                pos: IVec2::new(x, y),
            });
        }
        Ok(Snake {
            segments,
            state: Alive,
            last_tail_dir,
            player,
        })
    }
}

impl Serialize for InputPacket {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        match self {
            InputPacket::Direction { dir } => {
                result.push(0x00);
                match dir {
                    Dir::Up => {
                        result.push(0x00);
                    }
                    Dir::Down => {
                        result.push(0x01);
                    }
                    Dir::Left => {
                        result.push(0x02);
                    }
                    Dir::Right => {
                        result.push(0x03);
                    }
                }
            }
            InputPacket::Ping => {
                result.push(0x07);
            }
            InputPacket::Handshake(name) => {
                result.push(0x08);
                result.extend_from_slice(&(name.len() as u64).to_be_bytes());
                result.extend_from_slice(&name.clone().into_bytes());
            }
        }
        result
    }
}

impl Deserialize for InputPacket {
    fn deserialize(input: &[u8]) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let mut r = Reader::new(input);
        let itype = r.u8()?;
        match itype {
            0x00 => {
                let dir = r.u8()?;
                let dir = match dir {
                    0x00 => Dir::Up,
                    0x01 => Dir::Down,
                    0x02 => Dir::Left,
                    0x03 => Dir::Right,
                    _ => return Err(DecodeError::InvalidPacketType(0)),
                };
                Ok(InputPacket::Direction { dir })
            }
            0x07 => Ok(InputPacket::Ping),
            0x08 => {
                let string_length = r.u64_be()? as usize;
                let string_bytes = r.read(string_length)?;
                let name = match String::from_utf8(string_bytes.to_vec()) {
                    Ok(string) => string,
                    Err(_) => return Err(DecodeError::InvalidPacketType(0x08)),
                };
                Ok(InputPacket::Handshake(name))
            }
            _ => Err(DecodeError::InvalidPacketType(0x00)),
        }
    }
}
