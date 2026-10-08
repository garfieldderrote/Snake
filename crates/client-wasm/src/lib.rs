use std::{cell::RefCell, collections::HashMap};

use js_sys::Uint8Array;
use snake_core::{Board, Deserialize, Difference, InputPacket, OutputPacket, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket, console::log_1};

use crate::{draw::Draw, input::Input, world::ClientWorld};

mod draw;
mod input;
mod world;
thread_local! {
    static WEBSOCKET: RefCell<Option<WebSocket>> = const { RefCell::new(None) };
}
thread_local! {
    static PING_START: RefCell<Option<f64>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub fn start(name: String, ip: String) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let websocket = WebSocket::new(&format!("ws://{}", ip))?;
    websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    WEBSOCKET.with(|ws| {
        *ws.borrow_mut() = Some(websocket.clone());
    });

    start_ping_loop();

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

    /////DEBUG ///////////
    // let mut snakes = HashMap::new();
    // snakes.insert(
    //     PlayerId(0),
    //     Snake::new(IVec2::new(10, 10), Dir::Right, 5, PlayerId(0)),
    // );
    // let mut testWorld = ClientWorld::new(Board::new(25, 25, 0), snakes);
    // draw.draw_world(&mut testWorld);
    /////DEBUG ///////////

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

pub fn start_ping_loop() {
    let closure = Closure::<dyn FnMut()>::new(move || {
        PING_START.with(|start| {
            *start.borrow_mut() = Some(js_sys::Date::now());
        });
        WEBSOCKET.with(|ws| {
            if let Some(websocket) = ws.borrow().as_ref() {
                let packet = InputPacket::Ping;
                let bytes = packet.serialize();
                let array = Uint8Array::from(bytes.as_slice());
                match websocket.send_with_array_buffer(&array.buffer()) {
                    Ok(_) => {}
                    Err(error) => {
                        web_sys::console::error_1(&error);
                    }
                }
            }
        });
    });

    let window = match web_sys::window() {
        Some(window) => window,
        None => {
            web_sys::console::error_1(&"No window!".into());
            return;
        }
    };

    match window.set_interval_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        1000,
    ) {
        Ok(_) => {
            web_sys::console::log_1(&"Ping interval started".into());
        }
        Err(error) => {
            web_sys::console::error_1(&error);
        }
    }

    closure.forget();
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
        OutputPacket::Pong => {
            PING_START.with(|start| {
                let mut start = start.borrow_mut();
                if let Some(time) = start.take() {
                    let ping = js_sys::Date::now() - time;

                    update_ping_display(ping);
                }
            });
        }
    }
}

fn update_ping_display(ping: f64) {
    let Some(window) = web_sys::window() else {
        return;
    };

    let Some(document) = window.document() else {
        return;
    };

    let Some(element) = document.get_element_by_id("ping") else {
        return;
    };

    element.set_text_content(Some(&format!("{} ms", ping.round() as u32)));

    let color = if ping < 50.0 {
        "#4ade80" // green
    } else if ping < 100.0 {
        "#facc15" // yellow
    } else if ping < 200.0 {
        "#fb923c" // orange
    } else {
        "#ef4444" // red
    };

    let _ = element
        .dyn_ref::<web_sys::HtmlElement>()
        .unwrap()
        .style()
        .set_property("color", color);
}
