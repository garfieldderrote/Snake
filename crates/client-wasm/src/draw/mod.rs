use std::{
    cell::RefCell,
    collections::HashMap,
    f64::consts::{FRAC_PI_2, PI},
    rc::Rc,
};

mod animation;
mod draw_snake;
mod player_list;

use snake_core::{Board, Cell, Dir, IVec2, OutputPacket, PlayerId, Vec2};
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::{
    draw::{draw_snake::DrawSnake, player_list::PlayerListView},
    handle_received_packet, now_ms,
    tick::TickBuffer,
    world::ClientWorld,
};

pub struct Draw {
    context: CanvasRenderingContext2d,
    size: (u32, u32),
    cell_size: f64,
    world: ClientWorld,
    client_id: Option<PlayerId>,
    player_names: HashMap<PlayerId, String>,
    snake_drawer: DrawSnake,
    player_list_drawer: PlayerListView,
}

pub trait DrawWorld {
    fn get_height(&self) -> i32;
    fn get_width(&self) -> i32;
    fn get_cell_at(&self, pos: IVec2) -> Cell;
}

impl DrawWorld for ClientWorld {
    fn get_height(&self) -> i32 {
        self.board().get_height()
    }
    fn get_width(&self) -> i32 {
        self.board().get_width()
    }
    fn get_cell_at(&self, pos: IVec2) -> Cell {
        let mut cell = self.board().get_apple_at(pos);
        if cell != Cell::Empty {
            return cell;
        }
        for snake in self.snakes() {
            cell = snake.1.get_segment_pos_at(pos);
            if cell != Cell::Empty {
                return cell;
            }
        }
        Cell::Empty
    }
}
#[derive(Clone, Copy)]
struct Skin {
    head: [u8; 3],
    tail: [u8; 3],
}

impl Draw {
    pub fn new() -> Result<Self, JsValue> {
        let window = web_sys::window().expect("No global window found");
        let document = window.document().expect("Should have a document on window");

        // 2. Find the canvas element by its ID
        let canvas = document
            .get_element_by_id("canvas")
            .expect("Canvas element with id 'canvas' not found");

        // 3. Cast the element to an HtmlCanvasElement
        let canvas: HtmlCanvasElement = canvas
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("Element is not a canvas"))?;

        // 4. Get the 2D rendering context
        let context = canvas
            .get_context("2d")?
            .expect("Could not get 2D context")
            .dyn_into::<CanvasRenderingContext2d>()?;
        let size = (canvas.width(), canvas.height());
        Ok(Draw {
            snake_drawer: DrawSnake::new(context.clone(), size, 0.0),
            player_list_drawer: PlayerListView::new()?,
            context,
            size,
            cell_size: 0.0,
            world: ClientWorld::new(Board::new(0, 0, 0), HashMap::new()),
            client_id: None,
            player_names: HashMap::new(),
        })
    }
    pub fn set_client_id(&mut self, id: PlayerId) {
        self.client_id = Some(id);
    }
    pub fn get_client_id(&self) -> Option<PlayerId> {
        self.client_id
    }
    pub fn set_player_names(&mut self, playernames: Vec<(PlayerId, String)>) {
        let mut player_names = HashMap::new();
        for (player, name) in playernames {
            player_names.insert(player, name);
        }
        self.player_names = player_names;
    }
    pub fn draw_world(&mut self, world: &mut ClientWorld) {
        let cell_size = self.size.0 as f64 / world.get_width() as f64;
        self.cell_size = cell_size;
        self.snake_drawer.cell_size = cell_size;
        self.world = world.clone();
        //TODO: use the self.cell_size everywhere
        for (player, snake) in world.snakes() {
            self.snake_drawer.reset_tick(player, snake);
        }
        let _ = self.player_list_drawer.draw(&self.player_names);
    }
    pub fn draw_frame(&mut self, world: &ClientWorld, dt: f64) {
        let width = world.get_width();
        let height = world.get_height();
        // Background
        self.context.set_fill_style_str("#18202b");
        self.context
            .fill_rect(0.0, 0.0, self.size.0 as f64, self.size.1 as f64);

        // Subtle grid
        self.context.set_stroke_style_str("#222c3a");
        self.context.set_line_width(1.0);

        for x in 0..=width {
            let px = x as f64 * self.cell_size;
            self.context.begin_path();
            self.context.move_to(px, 0.0);
            self.context.line_to(px, self.size.1 as f64);
            self.context.stroke();
        }

        for y in 0..=height {
            let py = self.size.1 as f64 - y as f64 * self.cell_size;
            self.context.begin_path();
            self.context.move_to(0.0, py);
            self.context.line_to(self.size.0 as f64, py);
            self.context.stroke();
        }

        // Draw apples first so snakes appear on top.
        for y in 0..height {
            for x in 0..width {
                if matches!(world.get_cell_at(IVec2::new(x, y)), Cell::Apple) {
                    self.draw_apple(x, y, self.cell_size);
                }
            }
        }

        // Draw snakes using their segments.
        for snake in world.snakes().values() {
            self.snake_drawer.draw_snake(snake, dt);
        }
    }

    fn cell_center(&self, pos: &IVec2, cell_size: f64) -> (f64, f64) {
        (
            pos.x as f64 * cell_size + cell_size / 2.0,
            self.size.1 as f64 - pos.y as f64 * cell_size - cell_size / 2.0,
        )
    }

    fn draw_apple(&self, x: i32, y: i32, cell_size: f64) {
        let cx = x as f64 * cell_size + cell_size / 2.0;
        let cy = self.size.1 as f64 - y as f64 * cell_size - cell_size / 2.0;

        // Apple body
        self.context.set_fill_style_str("#ef4444");
        self.context.begin_path();
        self.context
            .arc(
                cx,
                cy + cell_size * 0.03,
                cell_size * 0.32,
                0.0,
                std::f64::consts::TAU,
            )
            .unwrap();
        self.context.fill();

        // Small highlight
        self.context.set_fill_style_str("#fca5a5");
        self.context.begin_path();
        self.context
            .arc(
                cx - cell_size * 0.11,
                cy - cell_size * 0.10,
                cell_size * 0.07,
                0.0,
                std::f64::consts::TAU,
            )
            .unwrap();
        self.context.fill();

        // Stem
        self.context.set_stroke_style_str("#713f12");
        self.context.set_line_width(cell_size * 0.07);
        self.context.begin_path();
        self.context.move_to(cx, cy - cell_size * 0.25);
        self.context
            .line_to(cx + cell_size * 0.08, cy - cell_size * 0.39);
        self.context.stroke();
    }
    pub fn update_snake_dir(&mut self, dir: Dir, player: Option<PlayerId>) {
        //let animation = self.head_animations.get_mut(&player).unwrap();
        if let Some(playerid) = player.or(self.get_client_id())
            && let Some(snake) = self.world.snakes().get(&playerid)
        {
            if !check_valid_input(snake.segments[0].dir, dir) {
                return;
            }
            let head_pixel_pos = self.cell_center(&snake.segments[0].pos, self.cell_size);
            let head_pixel_pos = Vec2::new(head_pixel_pos.0, head_pixel_pos.1);

            let halftile = match dir {
                Dir::Up => Vec2::new(0.0, self.cell_size / 2.0),
                Dir::Down => Vec2::new(0.0, -self.cell_size / 2.0),
                Dir::Left => Vec2::new(self.cell_size / 2.0, 0.0),
                Dir::Right => Vec2::new(-self.cell_size / 2.0, 0.0),
            };

            let rotation = match dir {
                Dir::Up => -FRAC_PI_2,
                Dir::Down => FRAC_PI_2,
                Dir::Left => PI,
                Dir::Right => 0.0,
            };
            if let Some(animation) = self.snake_drawer.head_animations.get_mut(&playerid) {
                if animation.retargeted {
                    return;
                }
                animation.target_offs = head_pixel_pos + halftile * -1.0;
                animation.target_rotation = rotation;
                animation.retargeted = true;
            }
        }
    }
}

type CallbackSlot = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

pub fn start_render_loop(
    world: Rc<RefCell<ClientWorld>>,
    draw: Rc<RefCell<Draw>>,
    tickbuffer: Rc<RefCell<TickBuffer<Vec<OutputPacket>>>>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let callback_slot: CallbackSlot = Rc::new(RefCell::new(None));

    let callback_slot_clone = Rc::clone(&callback_slot);
    let mut last_timestamp: Option<f64> = None;

    *callback_slot.borrow_mut() = Some(Closure::new(move |timestamp: f64| {
        let dt = last_timestamp.map(|last| timestamp - last).unwrap_or(0.0);
        // delta time in ms
        // 1. Play every tick that is due (usually none, sometimes one, rarely more).
        loop {
            // The borrow ends at the semicolon, before we touch world/draw.
            let due = tickbuffer.borrow_mut().pop_due(now_ms());
            let Some((_play_at, packets)) = due else {
                break;
            };
            for packet in packets {
                handle_received_packet(packet, &mut world.borrow_mut(), &mut draw.borrow_mut());
            }
        }
        {
            let world = world.borrow();
            draw.borrow_mut().draw_frame(&world, dt);
        }
        last_timestamp = Some(timestamp);

        if let Some(window) = web_sys::window() {
            let callback_slot = callback_slot_clone.borrow();

            if let Some(callback) = callback_slot.as_ref() {
                let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
            }
        }
    }));

    if let Some(callback) = callback_slot.borrow().as_ref() {
        let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
    }

    // Keep the callback alive for as long as the animation loop runs.
    std::mem::forget(callback_slot);
}

fn check_valid_input(dira: Dir, dirb: Dir) -> bool {
    !((dira == Dir::Up && dirb == Dir::Down)
        || (dira == Dir::Down && dirb == Dir::Up)
        || (dira == Dir::Left && dirb == Dir::Right)
        || (dira == Dir::Right && dirb == Dir::Left))
}
