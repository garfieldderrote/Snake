use serde::{Deserialize, Serialize};

use crate::server::game::PlayerId;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IVec2 {
    pub x: i32,
    pub y: i32,
}

impl IVec2 {
    pub fn new(x: i32, y: i32) -> IVec2 {
        IVec2 { x, y }
    }
    pub fn add(&self, other: &IVec2) -> IVec2 {
        IVec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, PartialEq)]
pub enum Cell {
    Empty,
    Apple,
    Snake(i32, PlayerId),
}
