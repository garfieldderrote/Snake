use std::{
    collections::HashMap,
    thread::sleep,
    time::{self, Duration, Instant},
};

use crate::{
    board::{Board, BoardSystem},
    draw::draw_board,
    input::{self, Input},
    networking::Network,
    snake::{Snake, SnakeSystem, SnakeWorld},
    util::{Cell, GameEnding, IVec2},
};

pub struct GameSystem;

pub struct Game<W: SnakeWorld> {
    world: W,
}

pub struct World {
    pub board: Board,
    pub snake: Snake,
}

pub struct MultiplayerWorld {
    pub board: Board,
    pub snakes: HashMap<PlayerId, Snake>,
}
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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
        self.snakes.insert(id, Snake::new(IVec2::new(10, 12), 3));
    }
}

// impl SnakeWorld for World {
//     fn snake(&self, _: PlayerId) -> Option<&Snake> {
//         Some(&self.snake)
//     }
//     fn snake_mut(&mut self, _: PlayerId) -> Option<&mut Snake> {
//         Some(&mut self.snake)
//     }
//     fn get_cell_at(&self, pos: IVec2) -> Cell {
//         let cell = self.board.get_apple_at(pos);
//         if cell != Cell::Empty {
//             return cell;
//         }
//
//         self.snake.get_segment_pos_at(pos)
//     }
//     fn is_valid_pos(&self, pos: IVec2) -> bool {
//         pos.x < self.board.get_width()
//             && pos.y < self.board.get_height()
//             && pos.x >= 0
//             && pos.y >= 0
//     }
//     fn remove_apple(&mut self, pos: IVec2) {
//         BoardSystem::remove_apple_at(self, pos);
//     }
//     fn get_width(&self) -> i32 {
//         self.board.get_width()
//     }
//     fn get_height(&self) -> i32 {
//         self.board.get_height()
//     }
//     fn board(&self) -> &Board {
//         &self.board
//     }
//     fn board_mut(&mut self) -> &mut Board {
//         &mut self.board
//     }
// }

impl GameSystem {
    pub fn run<W: SnakeWorld>(
        game: &mut Game<W>,
        input: &mut Input,
        network: &mut Network,
    ) -> GameEnding {
        let mut last_update = Instant::now();
        while !input.should_quit {
            sleep(time::Duration::from_millis(50));
            input.fetch();
            network.receive();
            if last_update.elapsed() >= Duration::from_millis(200) {
                GameSystem::tick(&mut game.world, input, network);
                // if !SnakeSystem::alive(game.world.snake(&PlayerId(0)).unwrap()) {
                //     return GameEnding::Failure;
                // }
                draw_board(&mut game.world);
                last_update = Instant::now();
            }
        }
        GameEnding::Misc
    }
    pub fn tick<W: SnakeWorld>(world: &mut W, input: &mut Input, network: &mut Network) {
        let input_dir = input.get_dir();
        let players = network.get_players().clone();
        for player in players {
            if world.snake(&player).is_none() {
                world.new_snake(player);
            }
            let input_dir = network.get_input_player(player);
            SnakeSystem::tick(world, input_dir, &player);
        }
        BoardSystem::tick(world);
    }
}

impl Game<MultiplayerWorld> {
    // pub fn new() -> Self {
    //     Game {
    //         world: World {
    //             board: Board::new(25, 25, 10),
    //             snake: Snake::new(IVec2::new(10, 12), 3),
    //         },
    //     }
    // }
    pub fn new() -> Self {
        let mut snakes: HashMap<PlayerId, Snake> = HashMap::new();

        Game {
            world: MultiplayerWorld {
                board: Board::new(25, 25, 100),
                snakes,
            },
        }
    }
}
