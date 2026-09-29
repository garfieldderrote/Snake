use std::{
    thread::sleep,
    time::{self, Duration, Instant},
};

use crate::{
    board::{Board, BoardSystem},
    draw::draw_board,
    input::Input,
    snake::{Snake, SnakeSystem, SnakeWorld},
    util::{Cell, GameEnding, IVec2},
};

pub struct GameSystem;

pub struct Game {
    world: World,
}

pub struct World {
    pub board: Board,
    pub snake: Snake,
}

impl SnakeWorld for World {
    fn snake(&self) -> &Snake {
        &self.snake
    }
    fn snake_mut(&mut self) -> &mut Snake {
        &mut self.snake
    }
    fn get_cell_at(&self, pos: IVec2) -> Cell {
        let cell = self.board.get_apple_at(pos);
        if cell != Cell::Empty {
            return cell;
        }

        self.snake.get_segment_pos_at(pos)
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

impl GameSystem {
    pub fn get_cell_at(world: &World, pos: IVec2) -> Cell {
        let cell = world.board.get_apple_at(pos);
        if cell != Cell::Empty {
            return cell;
        }

        world.snake.get_segment_pos_at(pos)
    }

    pub fn run(game: &mut Game, input: &mut Input) -> GameEnding {
        let mut last_update = Instant::now();
        while !input.should_quit {
            sleep(time::Duration::from_millis(50));
            input.fetch();
            if last_update.elapsed() >= Duration::from_millis(200) {
                GameSystem::tick(&mut game.world, input);
                if !SnakeSystem::alive(&game.world.snake) {
                    return GameEnding::Failure;
                }
                draw_board(&mut game.world);
                last_update = Instant::now();
            }
        }
        GameEnding::Misc
    }
    pub fn tick(world: &mut World, input: &mut Input) {
        let input_dir = input.get_dir();
        SnakeSystem::tick(world, input_dir);
        BoardSystem::tick(world);
    }
}

impl Game {
    pub fn new() -> Self {
        Game {
            world: World {
                board: Board::new(25, 25, 10),
                snake: Snake::new(IVec2::new(10, 12), 3),
            },
        }
    }
}
