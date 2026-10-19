use std::{
    cell::RefCell,
    collections::HashMap,
    f64::consts::{FRAC_PI_2, PI, TAU},
    rc::Rc,
};

use snake_core::{Board, Cell, Dir, IVec2, OutputPacket, PlayerId, Segment, Vec2};
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::{handle_received_packet, now_ms, tick::TickBuffer, world::ClientWorld};

struct HeadAnimation {
    start_offs: Vec2,
    pixel_offs: Vec2,
    target_offs: Vec2,
    tail_pos: Vec2,
    tail_target: Vec2,
    rotation: f64,
    target_rotation: f64,
    elapsed: f64,
    duration: f64,
    retargeted: bool,
}

pub struct Draw {
    context: CanvasRenderingContext2d,
    size: (u32, u32),
    head_animations: HashMap<PlayerId, HeadAnimation>,
    cell_size: f64,
    world: ClientWorld,
    client_id: Option<PlayerId>,
    player_names: HashMap<PlayerId, String>,
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

fn get_player_skin<'a>(player: &PlayerId) -> &'a str {
    let player_skins = ["blue", "orange", "purple", "teal", "yellow", "green"];
    player_skins[player.0 as usize % player_skins.len()]
}

fn skin_colors(name: &str) -> Skin {
    match name {
        "blue" => Skin {
            head: [0x46, 0x74, 0xE9],
            tail: [0x9C, 0xB8, 0xFF],
        },
        "orange" => Skin {
            head: [0xE8, 0x6A, 0x1F],
            tail: [0xFF, 0xB0, 0x7A],
        },
        "purple" => Skin {
            head: [0x7B, 0x4B, 0xE0],
            tail: [0xC4, 0xA8, 0xFF],
        },
        "teal" => Skin {
            head: [0x0F, 0x9D, 0x9A],
            tail: [0x84, 0xE0, 0xD9],
        },
        "yellow" => Skin {
            head: [0xF2, 0xA9, 0x00],
            tail: [0xFF, 0xE0, 0x82],
        },
        "green" => Skin {
            head: [0x1E, 0x8E, 0x3E],
            tail: [0x8A, 0xDB, 0x9E],
        },
        _ => Skin {
            head: [0x46, 0x74, 0xE9],
            tail: [0x9C, 0xB8, 0xFF],
        },
    }
}

fn mix(a: [u8; 3], b: [u8; 3], t: f64) -> String {
    let c = |i: usize| (a[i] as f64 + (b[i] as f64 - a[i] as f64) * t).round() as u8;
    format!("rgb({},{},{})", c(0), c(1), c(2))
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
            context,
            size,
            head_animations: HashMap::new(),
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
        for (player, name) in playernames {
            self.player_names.insert(player, name);
        }
    }
    pub fn draw_world(&mut self, world: &mut ClientWorld) {
        let cell_size = self.size.0 as f64 / world.get_width() as f64;
        self.cell_size = cell_size;
        self.world = world.clone();
        //TODO: use the self.cell_size everywhere
        for (player, snake) in world.snakes() {
            let halftile = match snake.segments[0].dir {
                Dir::Up => Vec2::new(0.0, cell_size / 2.0),
                Dir::Down => Vec2::new(0.0, -cell_size / 2.0),
                Dir::Left => Vec2::new(cell_size / 2.0, 0.0),
                Dir::Right => Vec2::new(-cell_size / 2.0, 0.0),
            };
            let rotation = match snake.segments[0].dir {
                Dir::Up => -FRAC_PI_2,
                Dir::Down => FRAC_PI_2,
                Dir::Left => PI,
                Dir::Right => 0.0,
            };

            let tail_target_dir = match snake.segments[snake.segments.len() - 1].dir {
                Dir::Up => Vec2::new(0.0, cell_size),
                Dir::Down => Vec2::new(0.0, -cell_size),
                Dir::Left => Vec2::new(cell_size, 0.0),
                Dir::Right => Vec2::new(-cell_size, 0.0),
            };

            let head_pixel_pos = self.cell_center(&snake.segments[0].pos, cell_size);
            let head_pixel_pos = Vec2::new(head_pixel_pos.0, head_pixel_pos.1);
            let tail_pixel_pos = self.cell_center(&snake.segments.last().unwrap().pos, cell_size);
            let tail_pixel_pos = Vec2::new(tail_pixel_pos.0, tail_pixel_pos.1);
            let tail_target_pos =
                self.cell_center(&snake.segments[snake.segments.len() - 1].pos, cell_size);
            let tail_target_pos = Vec2::new(tail_target_pos.0, tail_target_pos.1);
            self.head_animations
                .entry(*player)
                .and_modify(|anim| {
                    anim.target_offs = head_pixel_pos + halftile * -1.0;
                    anim.elapsed = 0.0;
                    anim.rotation = rotation;
                    anim.target_rotation = rotation;
                    anim.start_offs = head_pixel_pos + halftile;
                    anim.pixel_offs = head_pixel_pos + halftile;
                    anim.retargeted = false;
                    anim.tail_pos = tail_pixel_pos;
                    anim.tail_target = tail_target_pos - tail_target_dir;
                })
                .or_insert_with(|| {
                    HeadAnimation {
                        start_offs: head_pixel_pos + halftile,
                        pixel_offs: head_pixel_pos + halftile,
                        target_offs: head_pixel_pos + halftile * -1.0,
                        rotation,
                        target_rotation: rotation,
                        elapsed: 0.0,
                        duration: 150.0, // 200ms
                        retargeted: false,
                        tail_pos: tail_pixel_pos,
                        tail_target: tail_target_pos - tail_target_dir,
                    }
                });
        }
        //self.draw_frame(world, 0.0);
    }
    pub fn draw_frame(&mut self, world: &ClientWorld, dt: f64) {
        let cell_size = self.size.0 as f64 / world.get_width() as f64;
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
            let px = x as f64 * cell_size;
            self.context.begin_path();
            self.context.move_to(px, 0.0);
            self.context.line_to(px, self.size.1 as f64);
            self.context.stroke();
        }

        for y in 0..=height {
            let py = self.size.1 as f64 - y as f64 * cell_size;
            self.context.begin_path();
            self.context.move_to(0.0, py);
            self.context.line_to(self.size.0 as f64, py);
            self.context.stroke();
        }

        // Draw apples first so snakes appear on top.
        for y in 0..height {
            for x in 0..width {
                if matches!(world.get_cell_at(IVec2::new(x, y)), Cell::Apple) {
                    self.draw_apple(x, y, cell_size);
                }
            }
        }

        // Draw snakes using their segments.
        for (player, snake) in world.snakes() {
            let segments = snake.get_segments();

            if segments.is_empty() {
                continue;
            }
            if let Some(animation) = self.head_animations.get_mut(player) {
                update_head_animation(animation, dt);
                let pixel_offs = animation.pixel_offs;
                let rotation = animation.rotation;
                let tail_pos = animation.tail_pos;
                let color = skin_colors(get_player_skin(player));
                //self.draw_snake_outline(&segments, cell_size, pixel_offs);
                self.draw_snake_body(&segments, color, cell_size, pixel_offs, tail_pos);
                self.draw_snake_head(color, cell_size, pixel_offs, rotation);
            }
        }
    }

    fn draw_snake_body(
        &self,
        segments: &[Segment],
        skin: Skin, // was: color: &str
        cell_size: f64,
        pixel_offs: Vec2,
        tail_pos: Vec2,
    ) {
        if segments.len() < 2 {
            return;
        }

        // 1. Same path as before, head first.
        let mut pts: Vec<(f64, f64)> = Vec::with_capacity(segments.len() + 1);
        pts.push((pixel_offs.x, pixel_offs.y));
        for (i, segment) in segments.iter().enumerate() {
            pts.push(if i == segments.len() - 1 {
                (tail_pos.x, tail_pos.y)
            } else {
                self.cell_center(&segment.pos, cell_size)
            });
        }

        let total: f64 = pts
            .windows(2)
            .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
            .sum();
        if total < 1e-6 {
            return;
        }

        let head_r = cell_size * 0.34; // same thickness as your old 0.68 line width
        let tail_r = head_r * 0.70; // how thin the tail gets
        let step = (cell_size / 12.0).max(1.0); // distance between stamps
        //
        const MIN_FADE_CELLS: f64 = 12.0;
        let fade_len = (segments.len() as f64).max(MIN_FADE_CELLS) * cell_size;

        // 2. Sample points along the path: (x, y, t) with t = 0 at head, 1 at tail.
        let mut stamps: Vec<(f64, f64, f64)> = Vec::new();
        let mut walked = 0.0;
        for w in pts.windows(2) {
            let (x0, y0) = w[0];
            let (x1, y1) = w[1];
            let len = (x1 - x0).hypot(y1 - y0);
            let n = (len / step).ceil().max(1.0) as usize;
            for k in 0..n {
                let f = k as f64 / n as f64;
                stamps.push((
                    x0 + (x1 - x0) * f,
                    y0 + (y1 - y0) * f,
                    ((walked + len * f) / fade_len).min(1.0),
                ));
            }
            walked += len;
        }
        let &(lx, ly) = pts.last().unwrap();
        stamps.push((lx, ly, (walked / fade_len).min(1.0)));

        // 3. Draw tail -> head so the thick end overlaps the thin end.
        for &(x, y, t) in stamps.iter().rev() {
            let r = head_r + (tail_r - head_r) * t;
            self.context
                .set_fill_style_str(&mix(skin.head, skin.tail, t));
            self.context.begin_path();
            let _ = self.context.arc(x, y, r, 0.0, std::f64::consts::TAU);
            self.context.fill();
        }
    }
    fn draw_snake_head(&self, skin: Skin, cell_size: f64, pixel_offs: Vec2, rotation: f64) {
        let cx = pixel_offs.x;
        let cy = pixel_offs.y;

        let radius = cell_size * 0.34;

        // Eyes
        let eye_offset = cell_size * 0.28;
        let eye_radius = cell_size * 0.2;

        let nose_size = cell_size * 0.6;

        let forward_x = rotation.cos();
        let forward_y = rotation.sin();
        // Head
        self.context
            .set_stroke_style_str(&mix(skin.head, skin.head, 0.0));
        self.context.set_line_width(radius * 2.0);
        self.context.set_line_cap("round");
        self.context.set_line_join("round");

        self.context.begin_path();

        self.context.move_to(cx, cy);

        self.context
            .line_to(cx + forward_x * nose_size, cy + forward_y * nose_size);

        self.context.stroke();

        let (side_x, side_y) = (-forward_y, forward_x);

        for side in [-1.0, 1.0] {
            let ex = cx + forward_x * eye_offset * 0.5 + side_x * eye_offset * side;

            let ey = cy + forward_y * eye_offset * 0.5 + side_y * eye_offset * side;

            self.context
                .set_fill_style_str(&mix(skin.head, skin.head, 0.0));
            self.context.begin_path();
            self.context
                .arc(ex, ey, eye_radius * 1.3, 0.0, std::f64::consts::TAU)
                .unwrap();
            self.context.fill();

            self.context.set_fill_style_str("#ffffff");
            self.context.begin_path();
            self.context
                .arc(ex, ey, eye_radius, 0.0, std::f64::consts::TAU)
                .unwrap();
            self.context.fill();

            // Pupils
            self.context.set_fill_style_str("#111111");
            self.context.begin_path();
            self.context
                .arc(
                    ex + forward_x * eye_radius * 0.35,
                    ey + forward_y * eye_radius * 0.35,
                    eye_radius * 0.45,
                    0.0,
                    std::f64::consts::TAU,
                )
                .unwrap();
            self.context.fill();

            let ex = cx + forward_x * nose_size + side_x * eye_offset * 0.7 * side;

            let ey = cy + forward_y * nose_size + side_y * eye_offset * 0.7 * side;
            self.context.set_fill_style_str("#000000");
            self.context.begin_path();
            self.context
                .arc(
                    ex + forward_x * eye_radius * 0.35,
                    ey + forward_y * eye_radius * 0.35,
                    eye_radius * 0.3,
                    0.0,
                    std::f64::consts::TAU,
                )
                .unwrap();
            self.context.fill();
        }
    }
    fn draw_snake_outline(&self, segments: &[Segment], cell_size: f64, head_pixel_offs: Vec2) {
        if segments.len() < 2 {
            return;
        }

        self.context.set_stroke_style_str("#111827");
        self.context.set_line_width(cell_size * 0.82);
        self.context.set_line_cap("round");
        self.context.set_line_join("round");

        self.context.begin_path();

        // Start at the head so the outline connects underneath it.
        let x = head_pixel_offs.x;
        let y = head_pixel_offs.y;
        self.context.move_to(x, y);

        // Follow the entire snake.
        for (i, segment) in segments.iter().enumerate() {
            let (mut cx, mut cy) = self.cell_center(&segment.pos, cell_size);
            if i == 0 {
                cx = head_pixel_offs.x;
                cy = head_pixel_offs.y;
            }
            self.context.line_to(cx, cy);
        }

        self.context.stroke();

        let radius = cell_size * 0.40;

        let eye_offset = cell_size * 0.3;
        let eye_radius = cell_size * 0.2;
        let nose_size = cell_size * 0.6;

        let (forward_x, forward_y) = match segments[0].dir {
            Dir::Up => (0.0, -1.0),
            Dir::Down => (0.0, 1.0),
            Dir::Left => (-1.0, 0.0),
            Dir::Right => (1.0, 0.0),
        };

        self.context.set_stroke_style_str("#111827");
        self.context.set_line_width(radius * 2.1);
        self.context.set_line_cap("round");
        self.context.set_line_join("round");

        self.context.begin_path();

        self.context.move_to(x, y);

        self.context
            .line_to(x + forward_x * nose_size, y + forward_y * nose_size);

        self.context.stroke();

        let (side_x, side_y) = (-forward_y, forward_x);

        for side in [-1.0, 1.0] {
            let ex = x + forward_x * eye_offset * 0.5 + side_x * eye_offset * side;

            let ey = y + forward_y * eye_offset * 0.5 + side_y * eye_offset * side;
            self.context.set_fill_style_str("#111827");
            self.context.begin_path();
            self.context
                .arc(ex, ey, eye_radius * 1.8, 0.0, std::f64::consts::TAU)
                .unwrap();
            self.context.fill();
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
            if let Some(animation) = self.head_animations.get_mut(&playerid) {
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
pub fn start_render_loop(
    world: Rc<RefCell<ClientWorld>>,
    draw: Rc<RefCell<Draw>>,
    tickbuffer: Rc<RefCell<TickBuffer<Vec<OutputPacket>>>>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let callback_slot: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));

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

const TURN_TIME_FRACTION: f64 = 0.5;
const EPS: f64 = 1e-9;

fn wrap_pi(a: f64) -> f64 {
    (a + PI).rem_euclid(TAU) - PI
}

fn update_head_animation(anim: &mut HeadAnimation, dt: f64) {
    // Vector from where the head is now to where it should be.
    let dx = anim.target_offs.x - anim.pixel_offs.x;
    let dy = anim.target_offs.y - anim.pixel_offs.y;

    // ---- Position: constant velocity toward the *current* target ----
    // Each frame, cover dt / remaining of the distance that's left. If the
    // target never changes this is exactly constant speed and lands on the
    // target when elapsed == duration. If the target does change, we just
    // re-aim from wherever we are, so there is never a jump.
    let remaining = (anim.duration - anim.elapsed).max(0.0);
    anim.elapsed = (anim.elapsed + dt).min(anim.duration);

    if dt >= remaining {
        anim.pixel_offs.x = anim.target_offs.x;
        anim.pixel_offs.y = anim.target_offs.y;
    } else {
        let k = dt / remaining;
        anim.pixel_offs.x += dx * k;
        anim.pixel_offs.y += dy * k;
    }

    if remaining <= EPS || dt >= remaining {
        // End of tick, or already arrived: face the final direction exactly.
        anim.rotation = anim.target_rotation;
    } else {
        let diff = wrap_pi(anim.target_rotation - anim.rotation);
        let base_rate = FRAC_PI_2 / (anim.duration * TURN_TIME_FRACTION);
        let needed_rate = diff.abs() / remaining; // always finish by tick end
        let step = base_rate.max(needed_rate) * dt;
        anim.rotation = wrap_pi(anim.rotation + diff.clamp(-step, step));
    }

    // Tail Updating
    let delta = anim.tail_target - anim.tail_pos;

    if dt >= remaining {
        anim.tail_pos = anim.tail_target;
    } else {
        let k = dt / remaining;
        anim.tail_pos += delta * k;
    }
}

fn check_valid_input(dira: Dir, dirb: Dir) -> bool {
    !((dira == Dir::Up && dirb == Dir::Down)
        || (dira == Dir::Down && dirb == Dir::Up)
        || (dira == Dir::Left && dirb == Dir::Right)
        || (dira == Dir::Right && dirb == Dir::Left))
}
