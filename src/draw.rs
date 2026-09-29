use colored::Colorize;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::{
    snake::SnakeWorld,
    util::{Cell, IVec2},
};

pub fn draw_board<W: SnakeWorld>(world: &mut W) {
    _ = disable_raw_mode();
    print!("\x1B[3J\x1B[2J\x1B[H");
    for y in (0..world.get_height()).rev() {
        for x in 0..world.get_width() {
            match world.get_cell_at(IVec2::new(x, y)) {
                Cell::Empty => print!(". "),
                Cell::Apple => print!("{}", "0 ".red()),
                Cell::Snake(_) => print!("# "),
            }
        }
        println!();
    }
    _ = enable_raw_mode();
}
