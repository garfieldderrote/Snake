use colored::Colorize;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::{
    board::{self, Board},
    snake::{self, Snake},
    util::{Cell, IVec2},
};

pub trait DrawWorld {
    fn get_height(&self) -> i32;
    fn get_width(&self) -> i32;
    fn get_cell_at(&self, pos: IVec2) -> Cell;
}

pub struct ClientWorld {
    board: Board,
    snakes: Vec<Snake>,
}

impl ClientWorld {
    pub fn new(board: Board, snakes: Vec<Snake>) -> Self {
        ClientWorld { board, snakes }
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
            cell = snake.get_segment_pos_at(pos);
            if cell != Cell::Empty {
                return cell;
            }
        }
        Cell::Empty
    }
}

pub fn draw_board<W: DrawWorld>(world: &mut W) {
    _ = disable_raw_mode();
    print!("\x1B[3J\x1B[2J\x1B[H");
    for y in (0..world.get_height()).rev() {
        for x in 0..world.get_width() {
            match world.get_cell_at(IVec2::new(x, y)) {
                Cell::Empty => print!(". "),
                Cell::Apple => print!("{}", "0 ".red()),
                Cell::Snake(_) => print!("{}", "# ".blue()),
            }
        }
        println!();
    }
    _ = enable_raw_mode();
}
