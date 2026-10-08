use snake_core::{Cell, Dir, IVec2, PlayerId, Segment};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::world::ClientWorld;

pub struct Draw {
    context: CanvasRenderingContext2d,
    size: (u32, u32),
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

fn get_player_skin<'a>(player: &PlayerId) -> &'a str {
    let player_skins = ["blue", "green", "yellow"];
    player_skins[player.0 as usize % player_skins.len()]
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
        Ok(Draw { context, size })
    }
    // pub fn draw_world<W: DrawWorld>(&self, world: &mut W) {
    //     let cell_size = self.size.0 / world.get_width() as u32;
    //     for y in 0..world.get_height() {
    //         for x in 0..world.get_width() {
    //             match world.get_cell_at(IVec2::new(x, y)) {
    //                 Cell::Empty => self.context.set_fill_style_str("darkgrey"),
    //                 Cell::Apple => self.context.set_fill_style_str("red"),
    //                 Cell::Snake(_, player) => {
    //                     let color = get_player_skin(&player);
    //                     self.context.set_fill_style_str(color);
    //                 }
    //             }
    //             self.context.fill_rect(
    //                 cell_size as f64 * x as f64 + 1.0,
    //                 self.size.1 as f64 - (cell_size as f64 * y as f64) + 1.0,
    //                 cell_size as f64 - 2.0,
    //                 cell_size as f64 - 2.0,
    //             );
    //         }
    //     }
    // }

    pub fn draw_world(&self, world: &mut ClientWorld) {
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

            let color = get_player_skin(player);
            self.draw_snake_outline(&segments, cell_size);
            self.draw_snake_body(&segments, color, cell_size);
            self.draw_snake_head(&segments[0], color, cell_size);
        }
    }

    fn draw_snake_body(&self, segments: &[Segment], color: &str, cell_size: f64) {
        let radius = cell_size * 0.34;

        self.context.set_fill_style_str(color);

        for segment in segments.iter() {
            let (cx, cy) = self.cell_center(&segment.pos, cell_size);

            self.context.begin_path();
            self.context
                .arc(cx, cy, radius, 0.0, std::f64::consts::TAU)
                .unwrap();
            self.context.fill();
        }

        // Connect the segments with a thick line.
        //
        // This makes corners smooth instead of looking like
        // individual circles.
        self.context.set_stroke_style_str(color);
        self.context.set_line_width(cell_size * 0.68);
        self.context.set_line_cap("round");
        self.context.set_line_join("round");

        if segments.len() > 1 {
            self.context.begin_path();

            let (x, y) = self.cell_center(&segments[1].pos, cell_size);

            self.context.move_to(x, y);

            for segment in segments.iter() {
                let (x, y) = self.cell_center(&segment.pos, cell_size);
                self.context.line_to(x, y);
            }

            self.context.stroke();
        }
    }

    fn draw_snake_head(&self, segment: &Segment, color: &str, cell_size: f64) {
        let (cx, cy) = self.cell_center(&segment.pos, cell_size);

        let radius = cell_size * 0.34;

        // self.context.set_fill_style_str(color);
        // self.context.begin_path();
        // self.context
        //     .arc(cx, cy, radius, 0.0, std::f64::consts::TAU)
        //     .unwrap();
        // self.context.fill();

        // Eyes
        let eye_offset = cell_size * 0.28;
        let eye_radius = cell_size * 0.2;

        let nose_size = cell_size * 0.6;

        let (forward_x, forward_y) = match segment.dir {
            Dir::Up => (0.0, -1.0),
            Dir::Down => (0.0, 1.0),
            Dir::Left => (-1.0, 0.0),
            Dir::Right => (1.0, 0.0),
        };
        // Head
        self.context.set_stroke_style_str(color);
        self.context.set_line_width(radius * 2.0);
        self.context.set_line_cap("round");
        self.context.set_line_join("round");

        self.context.begin_path();

        let (x, y) = self.cell_center(&segment.pos, cell_size);

        self.context.move_to(x, y);

        self.context
            .line_to(x + forward_x * nose_size, y + forward_y * nose_size);

        self.context.stroke();

        let (side_x, side_y) = (-forward_y, forward_x);

        for side in [-1.0, 1.0] {
            let ex = cx + forward_x * eye_offset * 0.5 + side_x * eye_offset * side;

            let ey = cy + forward_y * eye_offset * 0.5 + side_y * eye_offset * side;

            self.context.set_fill_style_str(color);
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
    fn draw_snake_outline(&self, segments: &[Segment], cell_size: f64) {
        if segments.len() < 2 {
            return;
        }

        self.context.set_stroke_style_str("#111827");
        self.context.set_line_width(cell_size * 0.82);
        self.context.set_line_cap("round");
        self.context.set_line_join("round");

        self.context.begin_path();

        // Start at the head so the outline connects underneath it.
        let (x, y) = self.cell_center(&segments[0].pos, cell_size);
        self.context.move_to(x, y);

        // Follow the entire snake.
        for segment in segments.iter() {
            let (x, y) = self.cell_center(&segment.pos, cell_size);
            self.context.line_to(x, y);
        }

        self.context.stroke();

        let (cx, cy) = self.cell_center(&segments[0].pos, cell_size);

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

        let (x, y) = self.cell_center(&segments[0].pos, cell_size);

        self.context.move_to(x, y);

        self.context
            .line_to(x + forward_x * nose_size, y + forward_y * nose_size);

        self.context.stroke();

        let (side_x, side_y) = (-forward_y, forward_x);

        for side in [-1.0, 1.0] {
            let ex = cx + forward_x * eye_offset * 0.5 + side_x * eye_offset * side;

            let ey = cy + forward_y * eye_offset * 0.5 + side_y * eye_offset * side;
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
}
