use std::{
    collections::HashMap,
    thread::sleep,
    time::{self, Duration, Instant},
};

use rand::{RngExt, rng};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::{
    board::{Board, BoardSystem},
    server::network::Network,
    snake::{Snake, SnakeSystem, SnakeWorld},
    util::{Cell, Dir, IVec2},
};

pub struct GameSystem;

pub struct Game<W: SnakeWorld> {
    world: W,
}

pub struct MultiplayerWorld {
    pub board: Board,
    pub snakes: HashMap<PlayerId, Snake>,
}
#[derive(PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerId(pub u64);

impl SnakeWorld for MultiplayerWorld {
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
        BoardSystem::remove_apple_at(self, pos);
    }
    fn get_width(&self) -> i32 {
        self.board.get_width()
    }
    fn get_height(&self) -> i32 {
        self.board.get_height()
    }
    fn board(&self) -> &Board {
        &self.board
    }
    fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }
    fn new_snake(&mut self, id: PlayerId) {
        let mut rng = rng();
        assert!(self.get_width() >= 4);
        assert!(self.get_height() >= 4);
        let random_pos = IVec2::new(
            rng.random_range(2..(self.get_width() - 2)),
            rng.random_range(2..(self.get_height() - 2)),
        );
        let dir = match rng.random_range(0..4) {
            0 => Dir::Up,
            1 => Dir::Down,
            2 => Dir::Left,
            3 => Dir::Right,
            _ => Dir::Right,
        };

        self.snakes.insert(id, Snake::new(random_pos, dir, 3, id));
    }
    fn remove_snake(&mut self, id: &PlayerId) {
        self.snakes.remove(id);
    }
}

impl GameSystem {
    pub fn run<W: SnakeWorld>(
        game: &mut Game<W>,
        network: &mut Network,
        shutdown: CancellationToken,
    ) {
        let mut last_update = Instant::now();

        while !shutdown.is_cancelled() {
            sleep(time::Duration::from_millis(50));

            // fetch the network to store input in buffer
            // easiest way to get disconnected_players from network
            let disconnected_players = network.receive();
            for player in disconnected_players {
                game.world.remove_snake(&player);
            }
            if last_update.elapsed() >= Duration::from_millis(200) {
                GameSystem::tick(&mut game.world, network);
                last_update = Instant::now();
            }
        }
    }
    pub fn tick<W: SnakeWorld>(world: &mut W, network: &mut Network) {
        // get all connected players
        let players = network.get_players().clone();
        let mut snakes = Vec::new();
        for player in players {
            // spawn new snake
            if world.snake(&player).is_none() {
                world.new_snake(player);
            }

            // get buffered network input from player fetched from .receive()
            let input_dir = network.get_input_player(player);

            SnakeSystem::tick(world, input_dir, &player);

            // capture snakes in Vec<> for sending
            if world.snake(&player).is_some() {
                snakes.push(world.snake(&player).unwrap().clone());
            }
        }
        BoardSystem::tick(world);
        // sending the current board state
        network.send_game_state(world.board().clone(), snakes);
    }
}

impl Game<MultiplayerWorld> {
    pub fn new() -> Self {
        let snakes: HashMap<PlayerId, Snake> = HashMap::new();

        Game {
            world: MultiplayerWorld {
                board: Board::new(20, 20, 15),
                snakes,
            },
        }
    }
}
