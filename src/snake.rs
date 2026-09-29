use crate::util::{Cell, Dir, IVec2};

struct Segment {
    dir: Dir,
    pos: IVec2,
}

pub enum SnakeState {
    Alive,
    Crashed,
    Eating,
}

pub struct Snake {
    state: SnakeState,
    segments: Vec<Segment>,
    last_tail_dir: Dir,
}

pub trait SnakeWorld {
    fn snake(&self) -> &Snake;
    fn snake_mut(&mut self) -> &mut Snake;
    fn get_cell_at(&self, pos: IVec2) -> Cell;
    fn is_valid_pos(&self, pos: IVec2) -> bool;
    fn remove_apple(&mut self, pos: IVec2);
    fn get_width(&self) -> i32;
    fn get_height(&self) -> i32;
}

impl Snake {
    pub fn new(pos: IVec2, length: i32) -> Snake {
        let mut segments = vec![];
        for i in 0..length {
            segments.push(Segment {
                pos: IVec2::new(-i, 0).add(&pos),
                dir: Dir::Right,
            });
        }
        Snake {
            segments,
            state: SnakeState::Alive,
            last_tail_dir: Dir::Right,
        }
    }

    pub fn get_segments_pos(&self) -> Vec<IVec2> {
        self.segments.iter().map(|seg| seg.pos).collect()
    }
    pub fn get_segment_pos_at(&self, pos: IVec2) -> Cell {
        for seg in self.segments.iter().enumerate().rev() {
            if seg.1.pos == pos {
                return Cell::Snake(seg.0 as i32);
            }
        }
        Cell::Empty
    }
    pub fn get_head_pos(&self) -> IVec2 {
        self.segments[0].pos
    }
    pub fn get_head_dir(&self) -> Dir {
        self.segments[0].dir
    }
}

pub struct SnakeSystem;

impl SnakeSystem {
    pub fn tick<W: SnakeWorld>(world: &mut W, dir: Option<Dir>) {
        SnakeSystem::move_in_dir(world.snake_mut(), dir);
        SnakeSystem::update_state(world);
        SnakeSystem::handle_state(world);
    }
    pub fn move_in_dir(snake: &mut Snake, dir: Option<Dir>) {
        if let Some(dir) = dir {
            let head = snake.get_head_dir();
            if check_valid_input(dir, head) {
                snake.segments[0].dir = dir;
            }
        }
        if let Some(segment) = snake.segments.last() {
            snake.last_tail_dir = segment.dir;
        }
        let mut previous_dir = snake.get_head_dir();
        for segment in &mut snake.segments {
            let movement = match segment.dir {
                Dir::Up => IVec2::new(0, 1),
                Dir::Down => IVec2::new(0, -1),
                Dir::Right => IVec2::new(1, 0),
                Dir::Left => IVec2::new(-1, 0),
            };
            segment.pos = segment.pos.add(&movement);
            std::mem::swap(&mut segment.dir, &mut previous_dir);
        }
    }

    pub fn update_state<W: SnakeWorld>(world: &mut W) {
        let head_pos = world.snake().get_head_pos();
        if !world.is_valid_pos(head_pos) {
            world.snake_mut().state = SnakeState::Crashed;
            return;
        }
        match world.get_cell_at(head_pos) {
            Cell::Empty => world.snake_mut().state = SnakeState::Alive,
            Cell::Snake(i) => {
                if i != 0 {
                    world.snake_mut().state = SnakeState::Crashed
                } else {
                    world.snake_mut().state = SnakeState::Alive
                }
            }
            Cell::Apple => world.snake_mut().state = SnakeState::Eating,
        }
    }
    pub fn handle_state<W: SnakeWorld>(world: &mut W) {
        match world.snake().state {
            SnakeState::Eating => {
                world.remove_apple(world.snake().get_head_pos());
                SnakeSystem::grow(world.snake_mut());
            }
            SnakeState::Alive => {}
            SnakeState::Crashed => {}
        }
    }
    fn grow(snake: &mut Snake) {
        if let Some(tail) = snake.segments.last() {
            let dir = match snake.last_tail_dir {
                Dir::Up => IVec2::new(0, -1),
                Dir::Down => IVec2::new(0, 1),
                Dir::Right => IVec2::new(-1, 0),
                Dir::Left => IVec2::new(1, 0),
            };
            let new_pos = tail.pos.add(&dir);
            snake.segments.push(Segment {
                pos: new_pos,
                dir: snake.last_tail_dir,
            });
        }
    }
    pub fn alive(snake: &Snake) -> bool {
        match snake.state {
            SnakeState::Crashed => false,
            SnakeState::Alive => true,
            SnakeState::Eating => true,
        }
    }
}

fn check_valid_input(dira: Dir, dirb: Dir) -> bool {
    !((dira == Dir::Up && dirb == Dir::Down)
        || (dira == Dir::Down && dirb == Dir::Up)
        || (dira == Dir::Left && dirb == Dir::Right)
        || (dira == Dir::Right && dirb == Dir::Left))
}
