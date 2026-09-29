use crate::{
    game::{Game, GameSystem},
    input::Input,
    util::GameEnding,
};

mod board;
mod draw;
mod game;
mod input;
mod snake;
mod util;

fn main() {
    let mut game = Game::new();
    let mut input = Input::new();
    let result = GameSystem::run(&mut game, &mut input);
    Input::clean();
    match result {
        GameEnding::Victory => print_victory(),
        GameEnding::Failure => print_gameover(),
        GameEnding::Misc => {}
    }
}

fn print_victory() {
    println!("You Win!");
}

fn print_gameover() {
    println!("Game Over!");
}
