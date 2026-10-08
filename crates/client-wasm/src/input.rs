use js_sys::Uint8Array;
use snake_core::{Dir, InputPacket, Serialize};
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{KeyboardEvent, WebSocket, window};

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
            match key.as_str() {
                "w" | "a" | "s" | "d" => {
                    // Send the key through your websocket
                    let _ = websocket.send_with_str(&key);
                }
                _ => {}
            }
        });

        window()
            .unwrap()
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .unwrap();

        // IMPORTANT: keep the closure alive!
        closure.forget();
    }
}
