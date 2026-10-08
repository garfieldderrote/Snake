use std::collections::HashMap;

use snake_core::{Board, Deserialize, Difference, OutputPacket};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket, console::log_1};

use crate::{draw::Draw, input::Input, world::ClientWorld};

mod draw;
mod input;
mod world;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let websocket = WebSocket::new("ws://127.0.0.1:9000")?;
    websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let onopen = Closure::<dyn FnMut()>::new(|| {
        web_sys::console::log_1(&"WebSocket connected!".into());
    });

    websocket.set_onopen(Some(onopen.as_ref().unchecked_ref()));

    // Important: keep the closure alive.
    onopen.forget();
    Input::setup(websocket.clone());

    let mut world = ClientWorld::new(Board::new(0, 0, 0), HashMap::new());
    let draw = match Draw::new() {
        Ok(draw) => draw,
        Err(e) => {
            log_1(&e);
            return Err(e);
        }
    };
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Ok(array_buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
            let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();

            if let Ok(packet) = OutputPacket::deserialize(&bytes) {
                handle_received_packet(packet, &mut world, &draw);
            }
        }
    });

    websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    Ok(())
}

fn handle_received_packet(packet: OutputPacket, world: &mut ClientWorld, draw: &Draw) {
    match packet {
        OutputPacket::State {
            board_state,
            snakes,
        } => {
            *world = ClientWorld::new(board_state, snakes);
        }
        OutputPacket::Difference(Difference::RemovedApples(apples)) => {
            world.remove_apple(apples);
        }
        OutputPacket::Difference(Difference::AddedApples(apples)) => {
            world.add_apples(apples);
        }
        OutputPacket::Difference(Difference::SnakeMovements(movements)) => {
            for (player, dir) in movements {
                world.move_snake(player, dir);
            }
        }
        OutputPacket::Difference(Difference::AddSnake(snake)) => {
            world.add_snake(snake);
        }
        OutputPacket::Difference(Difference::RemoveSnake(player)) => {
            world.remove_snake(player);
        }
        OutputPacket::TickFinished => {
            draw.draw_world(world);
        }
    }
}
