use snake_core::{Cell, IVec2, PlayerId};
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

fn get_player_skin<'a>(player: PlayerId) -> &'a str {
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
    pub fn draw_world<W: DrawWorld>(&self, world: &mut W) {
        let cell_size = self.size.0 / world.get_width() as u32;
        for y in 0..world.get_height() {
            for x in 0..world.get_width() {
                match world.get_cell_at(IVec2::new(x, y)) {
                    Cell::Empty => self.context.set_fill_style_str("darkgrey"),
                    Cell::Apple => self.context.set_fill_style_str("red"),
                    Cell::Snake(_, player) => {
                        let color = get_player_skin(player);
                        self.context.set_fill_style_str(color);
                    }
                }
                self.context.fill_rect(
                    cell_size as f64 * x as f64 + 1.0,
                    self.size.1 as f64 - (cell_size as f64 * y as f64) + 1.0,
                    cell_size as f64 - 2.0,
                    cell_size as f64 - 2.0,
                );
            }
        }
    }
}
