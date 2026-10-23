use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use js_sys::Uint8Array;
use snake_core::{Dir, InputPacket, Serialize};
use wasm_bindgen::{
    JsCast,
    prelude::{Closure, wasm_bindgen},
};
use web_sys::{KeyboardEvent, WebSocket, window};

use crate::{
    WEBSOCKET,
    clock::{CLOCK, INPUTS, TICK_MS},
    draw::Draw,
    util::now_ms,
};

pub struct Input {}

impl Input {
    pub fn setup(draw: Rc<RefCell<Draw>>) {
        let closure = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            let key = event.key();

            let dir = match key.as_str() {
                "w" => Dir::Up,
                "s" => Dir::Down,
                "a" => Dir::Left,
                "d" => Dir::Right,
                _ => return,
            };
            draw.borrow_mut().update_snake_dir(dir, None);
            if let Some(t) = CLOCK.with(|c| c.borrow().predicted_server_time(now_ms())) {
                let tick = (t / TICK_MS).floor() as i64 + 1;
                INPUTS.with(|q| {
                    if q.borrow_mut().len() >= 2 {
                        return;
                    }
                    q.borrow_mut().push_back((0, dir))
                });
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
pub fn take_turn(tick: i64) -> Option<Dir> {
    INPUTS.with(|q| {
        let mut q = q.borrow_mut();
        let mut turn = None;
        if q.front().is_some_and(|&(tag, _)| tag <= tick) {
            turn = q.pop_front().map(|(_, dir)| dir);

            let packet = InputPacket::Direction {
                dir: turn.unwrap(),
                tick_number: tick as u64,
            };
            let array = Uint8Array::from(packet.serialize().as_slice());

            WEBSOCKET.with(|ws| {
                if let Some(websocket) = ws.borrow().as_ref() {
                    _ = websocket.send_with_array_buffer(&array.buffer());
                }
            });
        }
        turn
    })
}

#[wasm_bindgen]
pub fn input(dir: String) {
    // let dir = match dir.as_str() {
    //     "up" => Dir::Up,
    //     "down" => Dir::Down,
    //     "left" => Dir::Left,
    //     "right" => Dir::Right,
    //     _ => return,
    // };
    // let packet = InputPacket::Direction { dir };
    // WEBSOCKET.with(|ws| {
    //     if let Some(websocket) = ws.borrow().as_ref() {
    //         let array = Uint8Array::from(packet.serialize().as_slice());
    //
    //         if let Err(error) = websocket.send_with_array_buffer(&array.buffer()) {
    //             web_sys::console::error_1(&error);
    //         }
    //     }
    // });
}
