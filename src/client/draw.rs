use std::collections::HashMap;

use colored::{ColoredString, Colorize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::{
    board::Board,
    server::game::PlayerId,
    snake::{Snake, SnakeSystem, SnakeWorld},
    util::{Cell, Dir, IVec2},
};

pub trait DrawWorld {
    fn get_height(&self) -> i32;
    fn get_width(&self) -> i32;
    fn get_cell_at(&self, pos: IVec2) -> Cell;
}
#[derive(Clone)]
pub struct ClientWorld {
    board: Board,
    snakes: HashMap<PlayerId, Snake>,
}

impl ClientWorld {
    pub fn new(board: Board, snakes: HashMap<PlayerId, Snake>) -> Self {
        ClientWorld { board, snakes }
    }
}

impl ClientWorld {
    pub fn add_apples(&mut self, apples: Vec<IVec2>) {
        *self.board.get_apples_mut() = self
            .board
            .get_apples_mut()
            .iter()
            .chain(apples.iter())
            .copied()
            .collect();
    }
    pub fn remove_apple(&mut self, apples: Vec<IVec2>) {
        self.board
            .get_apples_mut()
            .retain(|a| !apples.iter().any(|b| a == b));
    }
    pub fn add_snake(&mut self, snake: Snake) {
        self.snakes.insert(snake.player, snake);
    }
    pub fn move_snake(&mut self, player: PlayerId, dir: Dir) {
        SnakeSystem::tick(self, Some(dir), &player);
    }
    pub fn remove_snake(&mut self, player: PlayerId) {
        self.snakes.remove(&player);
    }
}

impl SnakeWorld for ClientWorld {
    fn snake(&self, player: &PlayerId) -> Option<&Snake> {
        self.snakes.get(player)
    }
    fn snake_mut(&mut self, player: &PlayerId) -> Option<&mut Snake> {
        self.snakes.get_mut(player)
    }

    fn get_cell_at(&self, pos: IVec2) -> Cell {
        let mut cell = self.board.get_apple_at(pos);
        if cell != Cell::Empty {
            return cell;
        }
        for snake in self.snakes.values() {
            cell = snake.get_segment_pos_at(pos);
            if cell != Cell::Empty {
                return cell;
            }
        }
        Cell::Empty
    }
    fn get_cells_at(&self, pos: IVec2) -> Vec<Cell> {
        let mut cells = Vec::new();
        let cell = self.board.get_apple_at(pos);
        if cell != Cell::Empty {
            cells.push(cell);
        }
        for snake in self.snakes.values() {
            let cell = snake.get_segment_pos_at(pos);
            if cell != Cell::Empty {
                cells.push(cell);
            }
        }
        cells
    }
    fn is_valid_pos(&self, pos: IVec2) -> bool {
        pos.x < self.board.get_width()
            && pos.y < self.board.get_height()
            && pos.x >= 0
            && pos.y >= 0
    }
    fn remove_apple(&mut self, pos: IVec2) {
        self.board.get_apples_mut().retain(|a| a != &pos);
    }
    fn get_width(&self) -> i32 {
        self.board.get_width()
    }
    fn get_height(&self) -> i32 {
        self.board.get_height()
    }
}

impl DrawWorld for ClientWorld {
    fn get_height(&self) -> i32 {
        self.board.get_height()
    }
    fn get_width(&self) -> i32 {
        self.board.get_width()
    }
    fn get_cell_at(&self, pos: IVec2) -> Cell {
        let mut cell = self.board.get_apple_at(pos);
        if cell != Cell::Empty {
            return cell;
        }
        for snake in &self.snakes {
            cell = snake.1.get_segment_pos_at(pos);
            if cell != Cell::Empty {
                return cell;
            }
        }
        Cell::Empty
    }
}

fn get_player_skin(player: PlayerId) -> ColoredString {
    let player_skins = ["# ".blue(), "# ".green(), "# ".yellow()];
    player_skins[player.0 as usize % player_skins.len()].clone()
}

pub fn draw_board<W: DrawWorld>(world: &mut W) {
    _ = disable_raw_mode();
    print!("\x1B[3J\x1B[2J\x1B[H");
    for y in (0..world.get_height()).rev() {
        for x in 0..world.get_width() {
            match world.get_cell_at(IVec2::new(x, y)) {
                Cell::Empty => print!(". "),
                Cell::Apple => print!("{}", "0 ".red()),
                Cell::Snake(_, player) => print!("{}", get_player_skin(player)),
            }
        }
        println!();
    }
    _ = enable_raw_mode();
}
