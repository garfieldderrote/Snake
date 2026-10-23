use std::{cell::RefCell, rc::Rc};

use js_sys::Uint8Array;
use snake_core::{Deserialize, Difference, InputPacket, OutputPacket, Serialize, SnakeWorld};
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{MessageEvent, console::log_1};

use crate::{
    WEBSOCKET,
    clock::{CLOCK, PING_SENT},
    draw::{Draw, update_ping_display},
    util::now_ms,
    world::ClientWorld,
};
thread_local! {
    pub static INCOMING: RefCell<Vec<(f64,OutputPacket)>> = const { RefCell::new(Vec::new()) };
}

pub fn start_network(world: Rc<RefCell<ClientWorld>>, draw: Rc<RefCell<Draw>>) {
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Ok(array_buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
            let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();
            match OutputPacket::deserialize(&bytes) {
                Ok(OutputPacket::Pong(server_time)) => {
                    let t_recv = now_ms();
                    if let Some(t_sent) = PING_SENT.with(|q| q.borrow_mut().pop_front()) {
                        CLOCK.with(|c| {
                            c.borrow_mut()
                                .add_sample(t_sent, t_recv, server_time as f64)
                        });
                        update_ping_display(t_recv - t_sent);
                    }
                }
                Ok(packet) => INCOMING.with(|q| q.borrow_mut().push((now_ms(), packet))),
                Err(err) => {
                    web_sys::console::error_1(&format!("Invalid Packet: {:?}", err).into());
                }
            }
        }
    });

    WEBSOCKET.with(|w| {
        if let Some(websocket) = w.borrow_mut().as_ref() {
            websocket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        }
    });
    onmessage.forget();
}

pub fn handle_received_packet(packet: OutputPacket, world: &mut ClientWorld, draw: &mut Draw) {
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
        OutputPacket::Difference(Difference::SnakeMovement(player, dir, _)) => {
            if draw.get_client_id().is_some_and(|p| player == p) {
                return;
            }
            log_1(&format!("Move PlayerId {:?}", player).into());
            draw.update_snake_dir(dir, Some(player));
            world.move_snake(player, dir);
            let Some(snake) = world.snake(&player) else {
                return;
            };
        }
        OutputPacket::Difference(Difference::AddSnake(snake)) => {
            world.add_snake(snake);
        }
        OutputPacket::Difference(Difference::RemoveSnake(player)) => {
            world.remove_snake(player);
        }
        OutputPacket::TickFinished(sent_time) => {
            world.apply_movements();
            for (player, snake) in world.snakes() {
                draw.reset_animation(&player, snake);
            }
            //draw.draw_world(world);
        }
        OutputPacket::Handshake(playerid, playernames) => {
            log_1(&format!("Received Handshake, PlayerId = {:?}", playerid).into());
            draw.set_client_id(playerid);
            draw.set_player_names(playernames);
            log_1(&format!("[lol]player_id = {:?}", draw.get_client_id()).into());
        }
        OutputPacket::Points(player_points) => {
            draw.set_player_points(player_points);
        }
        _ => {}
    }
}

pub fn start_ping_loop() {
    let closure = Closure::<dyn FnMut()>::new(move || {
        WEBSOCKET.with(|ws| {
            if let Some(websocket) = ws.borrow().as_ref() {
                let packet = InputPacket::Ping;
                let bytes = packet.serialize();
                let array = Uint8Array::from(bytes.as_slice());
                let t0 = now_ms();
                if websocket.send_with_array_buffer(&array.buffer()).is_ok() {
                    PING_SENT.with(|q| q.borrow_mut().push_back(t0));
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
        30,
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
