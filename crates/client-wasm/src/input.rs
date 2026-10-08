use js_sys::Uint8Array;
use snake_core::{Dir, InputPacket, Serialize};
use wasm_bindgen::{
    JsCast,
    prelude::{Closure, wasm_bindgen},
};
use web_sys::{KeyboardEvent, WebSocket, window};

use crate::WEBSOCKET;

pub struct Input {}

impl Input {
    pub fn setup(websocket: WebSocket) {
        let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            let key = event.key();

            let dir = match key.as_str() {
                "w" => Dir::Up,
                "s" => Dir::Down,
                "a" => Dir::Left,
                "d" => Dir::Right,
                _ => return,
            };
            let packet = InputPacket::Direction { dir };
            let array = Uint8Array::from(packet.serialize().as_slice());
            _ = websocket.send_with_array_buffer(&array.buffer());
        });

        window()
            .unwrap()
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .unwrap();

        // IMPORTANT: keep the closure alive!
        closure.forget();
    }
}

#[wasm_bindgen]
pub fn input(dir: String) {
    let dir = match dir.as_str() {
        "up" => Dir::Up,
        "down" => Dir::Down,
        "left" => Dir::Left,
        "right" => Dir::Right,
        _ => return,
    };
    let packet = InputPacket::Direction { dir };
    WEBSOCKET.with(|ws| {
        if let Some(websocket) = ws.borrow().as_ref() {
            let array = Uint8Array::from(packet.serialize().as_slice());

            if let Err(error) = websocket.send_with_array_buffer(&array.buffer()) {
                web_sys::console::error_1(&error);
            }
        }
    });
}
