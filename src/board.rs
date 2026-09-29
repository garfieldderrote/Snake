use rand::RngExt;

use crate::{
    game::{GameSystem, World},
    util::{Cell, IVec2},
};

pub struct BoardSystem;

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
    pub fn get_apples(&self) -> Vec<IVec2> {
        self.apples.clone()
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
    pub fn tick(world: &mut World) {
        BoardSystem::spawn_apples(world);
    }
    fn spawn_apples(world: &mut World) {
        let mut rng = rand::rng();
        loop {
            if world.board.apples.len() >= world.board.num_apples as usize {
                break;
            }
            let pos = IVec2::new(
                rng.random_range(0..world.board.get_width()),
                rng.random_range(0..world.board.get_height()),
            );
            match GameSystem::get_cell_at(world, pos) {
                Cell::Empty => {}
                Cell::Apple => continue,
                Cell::Snake(_) => continue,
            };
            if world.board.apples.contains(&pos) {
                continue;
            }
            world.board.apples.push(pos);
        }
    }

    pub fn remove_apple_at(world: &mut World, pos: IVec2) {
        world.board.apples.retain(|apple| *apple != pos);
    }
}
