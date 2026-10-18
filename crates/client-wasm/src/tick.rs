use std::collections::VecDeque;

use web_sys::console::log_1;

pub struct TickBuffer<M> {
    interval: f64, // server tick interval in ms
    margin: f64,   // how late a tick may arrive without causing a stall, in ms
    lo: Option<f64>,
    last_play: f64,
    queue: VecDeque<(f64, M)>, // (play_at, message)
    pub underruns: u32,        // for tuning: how often a tick was later than the margin
}
pub struct PushInfo {
    pub slack: f64,
    pub jitter: f64,
}

impl<M> TickBuffer<M> {
    pub fn new(interval_ms: f64, margin_ms: f64) -> Self {
        Self {
            interval: interval_ms,
            margin: margin_ms,
            lo: None,
            last_play: 0.0,
            queue: VecDeque::new(),
            underruns: 0,
        }
    }

    /// Call from the websocket handler instead of applying the tick directly.
    /// `arrival` = performance.now() at the time the message was received.
    // pub fn push(&mut self, msg: M, arrival: f64, sent: f64) {
    //     // Where tick 0 would have been if every tick arrived perfectly evenly.
    //     let offset = arrival - sent;
    //
    //     let base = match self.base {
    //         // Normal case: drift slowly toward (typical arrival + margin).
    //         Some(b) if offset <= b => b + (offset + self.margin - b) * 0.02,
    //         // First tick, or a tick later than the margin allowed: re-anchor.
    //         Some(_) => {
    //             self.underruns += 1;
    //             log_1(&"Underrun!".into());
    //             offset + self.margin
    //         }
    //         None => offset + self.margin,
    //     };
    //     self.base = Some(base);
    //
    //     let play_at = base + sent;
    //     self.queue.push_back((play_at, msg));
    //
    //     self.n += 1; // use the server's tick number here if it sends one
    //
    //     log_1(
    //     &format!(
    //         "arrival={:.3}, n={}, offset={:.3}, base={:.3}, play_at={:.3}, queue={}, actual delay={}",
    //         arrival,
    //         self.n,
    //         offset,
    //         base,
    //         play_at,
    //         self.queue.len(),
    //         play_at - arrival
    //     )
    //     .into(),
    // );
    // }
    pub fn push(&mut self, msg: M, arrival: f64, sent: f64) -> PushInfo {
        let offset = arrival - sent;

        // lo = fastest path seen so far. Snap down to any faster arrival, creep up
        // slowly if the network really got slower. A single late tick barely moves it.
        let lo = match self.lo {
            Some(lo) if offset >= lo => lo + (offset - lo) * 0.05,
            _ => offset,
        };
        self.lo = Some(lo);

        let mut play_at = sent + lo + self.margin; // margin is added here, not stored in lo
        if play_at < arrival {
            self.underruns += 1;
            play_at = arrival; // late: play as soon as it is here, don't move the schedule
        }
        play_at = play_at.max(self.last_play); // never reorder
        self.last_play = play_at;
        self.queue.push_back((play_at, msg));

        log_1(
            &format!(
                "offset={offset:.1}, lo={lo:.1}, delay={:.1}, queue={}",
                play_at - arrival,
                self.queue.len()
            )
            .into(),
        );

        // in push(), before the `if play_at < arrival` clamp:
        let slack = sent + lo + self.margin - arrival; // negative = how late the tick was
        let jitter = offset - lo; // how far from the fastest path
        // ...at the end:
        PushInfo { slack, jitter }
    }
    // pub fn push(&mut self, msg: M, arrival: f64, next_tick: bool) {
    //     let offset = arrival - self.n as f64 * self.interval;
    //
    //     // Establish the playback timeline only once.
    //     let base = *self.base.get_or_insert(offset + self.margin);
    //
    //     let play_at = base + self.n as f64 * self.interval;
    //     self.queue.push_back((play_at, msg));
    //
    //     if next_tick {
    //         self.n += 1;
    //     }
    //     log_1(
    //         &format!(
    //             "arrival={:.3}, n={}, offset={:.3}, base={:.3}, play_at={:.3}, queue={}, actual delay={}",
    //             arrival,
    //             self.n,
    //             offset,
    //             base,
    //             play_at,
    //             self.queue.len(),
    //             play_at - arrival
    //         )
    //         .into(),
    //     );
    // }

    /// Call every frame (in a loop) with the rAF timestamp.
    pub fn pop_due(&mut self, now: f64) -> Option<(f64, M)> {
        match self.queue.front() {
            Some((play_at, _)) if *play_at <= now => self.queue.pop_front(),
            _ => None,
        }
    }
}
