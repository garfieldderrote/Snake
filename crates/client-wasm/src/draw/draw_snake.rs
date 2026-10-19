use std::{
    collections::HashMap,
    f64::consts::{FRAC_PI_2, PI},
};

use snake_core::{Dir, IVec2, PlayerId, Segment, Snake, Vec2};
use web_sys::CanvasRenderingContext2d;

use crate::draw::{
    Skin,
    animation::{HeadAnimation, update_head_animation},
};

pub struct DrawSnake {
    context: CanvasRenderingContext2d,
    size: (u32, u32),
    pub cell_size: f64,
    pub head_animations: HashMap<PlayerId, HeadAnimation>,
}

impl DrawSnake {
    pub fn new(context: CanvasRenderingContext2d, size: (u32, u32), cell_size: f64) -> Self {
        DrawSnake {
            context,
            size,
            cell_size,
            head_animations: HashMap::new(),
        }
    }
    pub fn draw_snake(&mut self, snake: &Snake, dt: f64) {
        let player = snake.player;

        let segments = snake.get_segments();

        if segments.is_empty() {
            return;
        }
        if let Some(animation) = self.head_animations.get_mut(&player) {
            update_head_animation(animation, dt);
            let animation = self.head_animations.get(&player).unwrap();
            let color = skin_colors(get_player_skin(&player));
            //self.draw_snake_outline(&segments, cell_size, pixel_offs);
            self.draw_snake_body(&segments, color, animation);
            self.draw_snake_head(color, animation);
        }
    }

    fn draw_snake_body(
        &self,
        segments: &[Segment],
        skin: Skin, // was: color: &str
        anim: &HeadAnimation,
    ) {
        if segments.len() < 2 {
            return;
        }

        // 1. Same path as before, head first.
        let mut pts: Vec<(f64, f64)> = Vec::with_capacity(segments.len() + 1);
        pts.push((anim.pixel_offs.x, anim.pixel_offs.y));
        for (i, segment) in segments.iter().enumerate() {
            pts.push(if i == segments.len() - 1 {
                (anim.tail_pos.x, anim.tail_pos.y)
            } else {
                self.cell_center(&segment.pos)
            });
        }

        let total: f64 = pts
            .windows(2)
            .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
            .sum();
        if total < 1e-6 {
            return;
        }

        let head_r = self.cell_size * 0.34; // same thickness as your old 0.68 line width
        let tail_r = head_r * 0.70; // how thin the tail gets
        let step = (self.cell_size / 12.0).max(1.0); // distance between stamps
        //
        const MIN_FADE_CELLS: f64 = 12.0;
        let fade_len = (segments.len() as f64).max(MIN_FADE_CELLS) * self.cell_size;

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

    pub fn draw_snake_head(&self, skin: Skin, anim: &HeadAnimation) {
        let cx = anim.pixel_offs.x;
        let cy = anim.pixel_offs.y;

        let radius = self.cell_size * 0.34;

        // Eyes
        let eye_offset = self.cell_size * 0.28;
        let eye_radius = self.cell_size * 0.2;

        let nose_size = self.cell_size * 0.6;

        let forward_x = anim.rotation.cos();
        let forward_y = anim.rotation.sin();
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

    pub fn reset_tick(&mut self, player: &PlayerId, snake: &Snake) {
        let halftile = match snake.segments[0].dir {
            Dir::Up => Vec2::new(0.0, self.cell_size / 2.0),
            Dir::Down => Vec2::new(0.0, -self.cell_size / 2.0),
            Dir::Left => Vec2::new(self.cell_size / 2.0, 0.0),
            Dir::Right => Vec2::new(-self.cell_size / 2.0, 0.0),
        };
        let rotation = match snake.segments[0].dir {
            Dir::Up => -FRAC_PI_2,
            Dir::Down => FRAC_PI_2,
            Dir::Left => PI,
            Dir::Right => 0.0,
        };

        let tail_target_dir = match snake.segments[snake.segments.len() - 1].dir {
            Dir::Up => Vec2::new(0.0, self.cell_size),
            Dir::Down => Vec2::new(0.0, -self.cell_size),
            Dir::Left => Vec2::new(self.cell_size, 0.0),
            Dir::Right => Vec2::new(-self.cell_size, 0.0),
        };

        let head_pixel_pos = self.cell_center(&snake.segments[0].pos);
        let head_pixel_pos = Vec2::new(head_pixel_pos.0, head_pixel_pos.1);
        let tail_pixel_pos = self.cell_center(&snake.segments.last().unwrap().pos);
        let tail_pixel_pos = Vec2::new(tail_pixel_pos.0, tail_pixel_pos.1);
        let tail_target_pos = self.cell_center(&snake.segments[snake.segments.len() - 1].pos);
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

    fn cell_center(&self, pos: &IVec2) -> (f64, f64) {
        (
            pos.x as f64 * self.cell_size + self.cell_size / 2.0,
            self.size.1 as f64 - pos.y as f64 * self.cell_size - self.cell_size / 2.0,
        )
    }
}

pub fn get_player_skin<'a>(player: &PlayerId) -> &'a str {
    let player_skins = ["blue", "orange", "purple", "teal", "yellow", "green"];
    player_skins[player.0 as usize % player_skins.len()]
}

pub fn skin_colors(name: &str) -> Skin {
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
