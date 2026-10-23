use std::{cell::RefCell, rc::Rc};

use snake_core::{OutputPacket, SnakeWorld};
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::console::log_1;

use crate::{
    clock::{CLOCK, INPUTS, TICK_MS, Ticker},
    draw::Draw,
    input::take_turn,
    network::{INCOMING, handle_received_packet},
    util::now_ms,
    world::ClientWorld,
};
#[derive(Default)]
struct TickStats {
    last_run: Option<f64>,
    intervals: Vec<f64>,
    lateness: Vec<f64>,
}

fn summarize(v: &[f64]) -> (f64, f64, f64) {
    let min = v.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (min, max, v.iter().sum::<f64>() / v.len() as f64)
}

impl TickStats {
    fn record(&mut self, now: f64, lateness_ms: f64) {
        if let Some(last) = self.last_run {
            self.intervals.push(now - last);
        }
        self.last_run = Some(now);
        self.lateness.push(lateness_ms);

        if self.intervals.len() >= 33 {
            // about 5 s
            let (min, max, mean) = summarize(&self.intervals);
            let (_, late_max, late_mean) = summarize(&self.lateness);
            dlog!(
                "[tick] interval min/mean/max = {min:.1}/{mean:.1}/{max:.1} ms | lateness mean/max = {late_mean:.1}/{late_max:.1} ms"
            );
            self.intervals.clear();
            self.lateness.clear();
        }
    }
}

pub fn start_game_loop(world: Rc<RefCell<ClientWorld>>, draw: Rc<RefCell<Draw>>) {
    let mut ticker = Ticker::default();
    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    let mut last_timestamp: Option<f64> = None;
    let mut stats = TickStats {
        last_run: None,
        intervals: Vec::new(),
        lateness: Vec::new(),
    };

    *g.borrow_mut() = Some(Closure::new(move |timestamp| {
        let mut world = world.borrow_mut();
        let mut draw = draw.borrow_mut();

        for (recv, packet) in INCOMING.with(|q| std::mem::take(&mut *q.borrow_mut())) {
            if let OutputPacket::TickFinished(sent_time) = &packet {
                let st = *sent_time as f64;
                let tick_no = (st / TICK_MS).round() as i64;
                if tick_no % 10 == 0
                    && let Some(server_now) = CLOCK.with(|c| c.borrow().predicted_server_time(recv))
                {
                    dlog!(
                        "[net] tick {tick_no}: phase = {:.1} ms, age at arrival = {:.1} ms",
                        st % TICK_MS,
                        server_now - st
                    );
                }
            }
            handle_received_packet(packet, &mut world, &mut draw);
        }

        let dt = last_timestamp.map(|last| timestamp - last).unwrap_or(0.0);
        last_timestamp = Some(timestamp);
        let t = CLOCK.with(|c| c.borrow().predicted_server_time(now_ms()));
        //log_1(&format!("t: {:?}", t).into());
        //log_1(&format!("player: {:?}", draw.get_client_id()).into());
        let now = now_ms();
        if let Some(t) = t
            && let Some(player) = draw.get_client_id()
        {
            let frame = ticker.poll(t);

            if frame.due > 0 {
                stats.record(now, frame.alpha * TICK_MS);
            }
            if frame.due > 1 {
                dlog!("[tick] catching up {} ticks", frame.due);
            }

            for k in (frame.tick - frame.due + 1)..=frame.tick {
                // apply inputs that belong to tick k or earlier
                let dir = take_turn(k);
                world.tick_snake(player, dir);
                let Some(snake) = world.snake(&player) else {
                    break;
                };
                draw.reset_animation(&player, snake);
                draw.draw_world(&mut world);

                // advance one tick locally
                // world.apply_movements();
            }
            draw.draw_frame(&world, dt);
        }
        //log_1(&"New Frame!".into());
        request_animation_frame(f.borrow().as_ref().unwrap());
    }));
    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}
