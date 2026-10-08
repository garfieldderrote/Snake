use std::{
    collections::HashMap,
    thread::sleep,
    time::{self, Duration, Instant},
};

use rand::{RngExt, rng};
use snake_core::{
    Board, BoardSystem, Cell, Dir, GameWorld, IVec2, PlayerId, Snake, SnakeSystem, SnakeWorld,
};
use tokio_util::sync::CancellationToken;
use tracy_client::span;

use crate::network::Network;

pub struct GameSystem;

pub struct Game<W: SnakeWorld> {
    world: W,
}

pub struct MultiplayerWorld {
    pub board: Board,
    pub snakes: HashMap<PlayerId, Snake>,
}

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
}

impl GameWorld for MultiplayerWorld {
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
    fn get_snake_hashmap(&self) -> &HashMap<PlayerId, Snake> {
        &self.snakes
    }
}

impl GameSystem {
    pub fn run<W: GameWorld + SnakeWorld>(
        game: &mut Game<W>,
        network: &mut Network,
        shutdown: CancellationToken,
    ) {
        let mut last_update = Instant::now();

        // debug

        while !shutdown.is_cancelled() {
            sleep(time::Duration::from_millis(50));

            // fetch the network to store input in buffer
            // easiest way to get disconnected_players from network
            let network_fetch_span = span!("NetworkFetch");
            network_fetch_span.emit_color(0xFF0000);
            let (disconnected_players, connected_players) = network.receive();
            for player in disconnected_players {
                game.world.remove_snake(&player);
                network.remove_snake(player);
            }
            for _player in connected_players {
                network.send_game_state(
                    game.world.board().clone(),
                    game.world.get_snake_hashmap().clone(),
                );
            }
            drop(network_fetch_span);

            if last_update.elapsed() >= Duration::from_millis(200) {
                let tick_zone = span!("Tick");
                tick_zone.emit_color(0x00FF00);
                GameSystem::tick(&mut game.world, network);
                last_update = Instant::now();
            }
        }
    }
    pub fn tick<W: GameWorld + SnakeWorld>(world: &mut W, network: &mut Network) {
        // get all connected players
        let mut snake_movement = HashMap::new();
        let players = network.get_players().clone();
        for player in players {
            // spawn new snake
            if world.snake(&player).is_none() {
                world.new_snake(player);
                network.add_snake(world.snake(&player).unwrap().clone());
            }
            // get buffered network input from player fetched from .receive()
            let input_dir = network.get_input_player(player);
            // store input dir for transmision
            if let Some(dir) = input_dir {
                snake_movement.insert(player, dir);
            } else {
                snake_movement.insert(player, world.snake(&player).unwrap().get_dir());
            }
            SnakeSystem::tick(world, input_dir, &player);
            if !SnakeSystem::alive(world.snake(&player).unwrap()) {
                world.remove_snake(&player);
                network.remove_snake(player);
            }
        }
        network.snake_movements(snake_movement);
        let board = world.board().clone();

        BoardSystem::tick(world);

        let added_apples: Vec<IVec2> = world
            .board()
            .get_apples()
            .iter()
            .filter(|a| !board.get_apples().contains(*a))
            .copied()
            .collect();
        let removed_apples: Vec<IVec2> = board
            .get_apples()
            .iter()
            .filter(|a| !world.board().get_apples().contains(*a))
            .copied()
            .collect();
        if !removed_apples.is_empty() {
            network.remove_apples(removed_apples);
        }
        if !added_apples.is_empty() {
            network.add_apples(added_apples);
        }
        network.send_finished();

        // sending the current board state
    }
}

impl Game<MultiplayerWorld> {
    pub fn new() -> Self {
        let snakes: HashMap<PlayerId, Snake> = HashMap::new();

        Game {
            world: MultiplayerWorld {
                board: Board::new(25, 25, 15),
                snakes,
            },
        }
    }
}

impl Default for Game<MultiplayerWorld> {
    fn default() -> Self {
        Self::new()
    }
}
