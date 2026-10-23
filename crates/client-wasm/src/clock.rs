use snake_core::Dir;
use std::{cell::RefCell, collections::VecDeque};

pub const TICK_MS: f64 = 150.0;
const MARGIN_MS: f64 = 8.0; // extra lead so inputs don't land just after the server tick
const WINDOW: usize = 40; // ~1.2 s of samples at one ping per 30 ms
const MAX_CATCH_UP: i64 = 3;

thread_local! {
    pub static CLOCK: RefCell<ClockSync> = RefCell::new(ClockSync::new());
    pub static PING_SENT: RefCell<VecDeque<f64>> = const { RefCell::new(VecDeque::new()) };
    pub static INPUTS: RefCell<VecDeque<(i64, Dir)>> = const { RefCell::new(VecDeque::new()) };
}

pub struct ClockSync {
    samples: VecDeque<(f64, f64)>, // (rtt, offset)
    offset: f64,                   // server_time ≈ client_now + offset
    rtt: f64,
    ready: bool,
    count: u64,
}

impl ClockSync {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::new(),
            offset: 0.0,
            rtt: 0.0,
            ready: false,
            count: 0,
        }
    }

    pub fn add_sample(&mut self, t_sent: f64, t_recv: f64, server_time: f64) {
        let rtt = t_recv - t_sent;
        // the server stamped its reply about rtt/2 before we received it
        let offset = server_time + rtt / 2.0 - t_recv;

        self.samples.push_back((rtt, offset));
        if self.samples.len() > WINDOW {
            self.samples.pop_front();
        }

        // The lowest-rtt sample had the least queueing, so the
        // "symmetric path" assumption holds best for it.
        let &(best_rtt, best_offset) = self
            .samples
            .iter()
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap();
        self.rtt = best_rtt;

        if !self.ready || (best_offset - self.offset).abs() > 50.0 {
            self.offset = best_offset; // first sample or big jump: snap
            self.ready = true;
            // inside add_sample, in the snap branch:
            if self.ready {
                dlog!("[clock] snap {:+.1} ms", best_offset - self.offset);
            }
        } else {
            self.offset += (best_offset - self.offset) * 0.05; // otherwise slew gently
            //
            // ...and at the end:
        }

        self.count += 1;
        if self.count % 33 == 0 {
            dlog!(
                "[clock] offset = {:.2} ms, rtt = {:.1} ms (window min)",
                self.offset,
                self.rtt
            );
        }
    }

    /// Server clock at the moment a packet sent right now would arrive.
    pub fn predicted_server_time(&self, now: f64) -> Option<f64> {
        self.ready
            .then(|| now + self.offset + self.rtt / 2.0 + MARGIN_MS)
    }
}

pub struct Frame {
    pub tick: i64,  // current client tick
    pub due: i64,   // how many ticks to simulate this frame (usually 0 or 1)
    pub alpha: f64, // 0..1 progress through the current tick, for rendering
}

#[derive(Default)]
pub struct Ticker {
    last: Option<i64>,
}

impl Ticker {
    pub fn poll(&mut self, t: f64) -> Frame {
        let exact = t / TICK_MS;
        let last = self.last.unwrap_or(exact.floor() as i64);
        // never go backwards if the estimate wobbles a ms or two
        let tick = (exact.floor() as i64).max(last);
        // after a background tab, don't replay a pile of ticks
        let due = (tick - last).min(MAX_CATCH_UP);
        self.last = Some(tick);
        Frame {
            tick,
            due,
            alpha: (exact - tick as f64).max(0.0),
        }
    }
}
