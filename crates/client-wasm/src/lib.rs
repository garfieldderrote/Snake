use std::{cell::RefCell, collections::HashMap, rc::Rc};

use js_sys::Uint8Array;
use snake_core::{Board, InputPacket, Serialize};
use wasm_bindgen::{
    JsCast, JsValue,
    prelude::{Closure, wasm_bindgen},
};
use web_sys::{WebSocket, console::log_1};

use crate::{
    draw::Draw,
    input::Input,
    network::{start_network, start_ping_loop},
    tickloop::start_game_loop,
    world::ClientWorld,
};
macro_rules! dlog {
    ($($t:tt)*) => {
        if cfg!(debug_assertions) {
            web_sys::console::log_1(&format!($($t)*).into());
        }
    };
}

mod clock;
mod draw;
mod input;
mod network;
mod tick;
mod tickloop;
mod util;
mod world;

thread_local! {
    static WEBSOCKET: RefCell<Option<WebSocket>> = const { RefCell::new(None) };
}
#[wasm_bindgen]
pub fn start(name: String, ip: String) -> Result<(), JsValue> {
    // init websocket
    let websocket = WebSocket::new(&format!("ws://{}:9000", ip))?;
    websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);
    WEBSOCKET.with(|w| {
        *w.borrow_mut() = Some(websocket);
    });

    let onopen = Closure::<dyn FnMut()>::new(move || {
        web_sys::console::log_1(&"WebSocket connected!".into());
        let packet = InputPacket::Handshake(name.clone());
        log_1(&"Sent Handshake!".into());
        WEBSOCKET.with(|ws| {
            if let Some(websocket) = ws.borrow().as_ref() {
                let array = Uint8Array::from(packet.serialize().as_slice());

                if let Err(error) = websocket.send_with_array_buffer(&array.buffer()) {
                    web_sys::console::error_1(&error);
                }
            }
        });
    });

    WEBSOCKET.with(|w| {
        if let Some(websocket) = w.borrow_mut().as_ref() {
            websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        }
    });

    // Important: keep the closure alive.
    onopen.forget();

    let world = Rc::new(RefCell::new(ClientWorld::new(
        Board::new(0, 0, 0),
        HashMap::new(),
    )));
    let draw = Rc::new(RefCell::new(Draw::new()?));
    Input::setup(draw.clone());

    start_ping_loop();

    start_network(world.clone(), draw.clone());
    start_game_loop(world.clone(), draw.clone());
    Ok(())
}
