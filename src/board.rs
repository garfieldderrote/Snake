use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::{
    snake::SnakeWorld,
    util::{Cell, IVec2},
};

pub struct BoardSystem;

#[derive(Serialize, Deserialize, Clone)]
pub struct Board {
    size: (i32, i32),
    apples: Vec<IVec2>,
    num_apples: i32,
}

impl Board {
    pub fn new(width: i32, height: i32, num_apples: i32) -> Board {
        let apples = Vec::new();
        Board {
            size: (width, height),
            apples,
            num_apples,
        }
    }
    pub fn get_apple_at(&self, pos: IVec2) -> Cell {
        if self.apples.contains(&pos) {
            return Cell::Apple;
        }
        Cell::Empty
    }
    pub fn get_width(&self) -> i32 {
        self.size.0
    }
    pub fn get_height(&self) -> i32 {
        self.size.1
    }
}
impl BoardSystem {
    pub fn tick<W: SnakeWorld>(world: &mut W) {
        BoardSystem::spawn_apples(world);
    }
    fn spawn_apples<W: SnakeWorld>(world: &mut W) {
        let mut rng = rand::rng();
        loop {
            if world.board().apples.len() >= world.board().num_apples as usize {
                break;
            }
            let pos = IVec2::new(
                rng.random_range(0..world.get_width()),
                rng.random_range(0..world.get_height()),
            );
            match world.get_cell_at(pos) {
                Cell::Empty => {}
                _ => continue,
            };
            if world.board().apples.contains(&pos) {
                continue;
            }
            world.board_mut().apples.push(pos);
        }
    }

    pub fn remove_apple_at<W: SnakeWorld>(world: &mut W, pos: IVec2) {
        world.board_mut().apples.retain(|apple| *apple != pos);
    }
}
