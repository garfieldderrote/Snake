use std::collections::VecDeque;

pub struct TickBuffer<M> {
    margin: f64, // how late a tick may arrive without causing a stall, in ms
    lo: Option<f64>,
    last_play: f64,
    queue: VecDeque<(f64, M)>, // (play_at, message)
    pub underruns: u32,        // for tuning: how often a tick was later than the margin
}

impl<M> TickBuffer<M> {
    pub fn new(margin_ms: f64) -> Self {
        Self {
            margin: margin_ms,
            lo: None,
            last_play: 0.0,
            queue: VecDeque::new(),
            underruns: 0,
        }
    }

    pub fn push(&mut self, msg: M, arrival: f64, sent: f64) {
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
    }

    /// Call every frame (in a loop) with the rAF timestamp.
    pub fn pop_due(&mut self, now: f64) -> Option<(f64, M)> {
        match self.queue.front() {
            Some((play_at, _)) if *play_at <= now => self.queue.pop_front(),
            _ => None,
        }
    }
}
