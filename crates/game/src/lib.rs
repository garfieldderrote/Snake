mod board;
mod protocol;
mod snake;
mod util;

use std::collections::HashMap;

pub use board::*;
pub use protocol::*;
pub use snake::*;
pub use util::*;
pub trait GameWorld {
    fn new_snake(&mut self, id: PlayerId);
    fn remove_snake(&mut self, id: &PlayerId);
    fn get_snake_hashmap(&self) -> &HashMap<PlayerId, Snake>;
    fn board(&self) -> &Board;
    fn board_mut(&mut self) -> &mut Board;
}
