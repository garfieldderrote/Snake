use std::collections::HashMap;

use snake_core::{PlayerId, Vec2};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::draw::{
    animation::HeadAnimation,
    draw_snake::{DrawSnake, get_player_skin, skin_colors},
};

const ROW_H: f64 = 40.0;
const PAD: f64 = 10.0;
const HEAD_SIZE: f64 = 30.0;
const LIST_W: f64 = 220.0;
const NAME_GAP: f64 = 10.0;
const FONT: &str = "bold 20px Arial, sans-serif";

pub struct PlayerListView {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
}

impl PlayerListView {
    pub fn new() -> Result<Self, JsValue> {
        let window = web_sys::window().expect("No global window found");
        let document = window.document().expect("Should have a document on window");

        // 2. Find the canvas element by its ID
        let canvas = document
            .get_element_by_id("players-canvas")
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
        Ok(PlayerListView {
            canvas,
            ctx: context,
        })
    }
    pub fn draw(&self, players: &HashMap<PlayerId, String>) -> Result<(), JsValue> {
        let dpr = web_sys::window().unwrap().device_pixel_ratio();
        let h = players.len() as f64 * ROW_H + PAD * 2.0;

        // Setting width/height clears the canvas, so only do it when the size changes
        let (pw, ph) = ((LIST_W * dpr).round() as u32, (h * dpr).round() as u32);
        if self.canvas.width() != pw || self.canvas.height() != ph {
            self.canvas.set_width(pw);
            self.canvas.set_height(ph);
            let style = self.canvas.style();
            style.set_property("width", &format!("{LIST_W}px"))?;
            style.set_property("height", &format!("{h}px"))?;
        }

        // Draw in CSS pixels; dpr keeps it sharp
        self.ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0)?;
        self.ctx.clear_rect(0.0, 0.0, LIST_W, h);
        self.ctx.set_font("14px sans-serif");
        self.ctx.set_text_baseline("middle");
        self.ctx.set_fill_style_str("#fff");

        let snake_draw = DrawSnake::new(self.ctx.clone(), (0, 0), HEAD_SIZE);

        for (i, (player, name)) in players.iter().enumerate() {
            let color = skin_colors(get_player_skin(player));
            let cy = PAD + i as f64 * (ROW_H + NAME_GAP) + ROW_H / 2.0;
            let anim = HeadAnimation::default(Vec2::new(PAD + 8.0, cy));
            snake_draw.draw_snake_head(color, &anim);
            self.ctx.set_font(FONT);
            self.ctx.set_text_align("left");
            self.ctx.set_text_baseline("middle");
            self.ctx.set_fill_style_str("#fff");
            self.ctx.fill_text(name, PAD + HEAD_SIZE + 20.0, cy)?;
        }
        Ok(())
    }
}
