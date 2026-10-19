use std::f64::consts::{FRAC_PI_2, PI, TAU};

use snake_core::Vec2;

pub struct HeadAnimation {
    pub start_offs: Vec2,
    pub pixel_offs: Vec2,
    pub target_offs: Vec2,
    pub tail_pos: Vec2,
    pub tail_target: Vec2,
    pub rotation: f64,
    pub target_rotation: f64,
    pub elapsed: f64,
    pub duration: f64,
    pub retargeted: bool,
}

impl HeadAnimation {
    // Returns all default with pixel_offs set to pos
    pub fn default(pos: Vec2) -> Self {
        HeadAnimation {
            start_offs: Vec2::new(0.0, 0.0),
            pixel_offs: pos,
            target_offs: Vec2::new(0.0, 0.0),
            tail_pos: Vec2::new(0.0, 0.0),
            tail_target: Vec2::new(0.0, 0.0),
            rotation: 0.0,
            target_rotation: 0.0,
            elapsed: 0.0,
            duration: 0.0,
            retargeted: false,
        }
    }
}

const TURN_TIME_FRACTION: f64 = 0.5;
const EPS: f64 = 1e-9;

fn wrap_pi(a: f64) -> f64 {
    (a + PI).rem_euclid(TAU) - PI
}

pub fn update_head_animation(anim: &mut HeadAnimation, dt: f64) {
    // Vector from where the head is now to where it should be.
    let dx = anim.target_offs.x - anim.pixel_offs.x;
    let dy = anim.target_offs.y - anim.pixel_offs.y;

    // ---- Position: constant velocity toward the *current* target ----
    // Each frame, cover dt / remaining of the distance that's left. If the
    // target never changes this is exactly constant speed and lands on the
    // target when elapsed == duration. If the target does change, we just
    // re-aim from wherever we are, so there is never a jump.
    let remaining = (anim.duration - anim.elapsed).max(0.0);
    anim.elapsed = (anim.elapsed + dt).min(anim.duration);

    if dt >= remaining {
        anim.pixel_offs.x = anim.target_offs.x;
        anim.pixel_offs.y = anim.target_offs.y;
    } else {
        let k = dt / remaining;
        anim.pixel_offs.x += dx * k;
        anim.pixel_offs.y += dy * k;
    }

    if remaining <= EPS || dt >= remaining {
        // End of tick, or already arrived: face the final direction exactly.
        anim.rotation = anim.target_rotation;
    } else {
        let diff = wrap_pi(anim.target_rotation - anim.rotation);
        let base_rate = FRAC_PI_2 / (anim.duration * TURN_TIME_FRACTION);
        let needed_rate = diff.abs() / remaining; // always finish by tick end
        let step = base_rate.max(needed_rate) * dt;
        anim.rotation = wrap_pi(anim.rotation + diff.clamp(-step, step));
    }

    // Tail Updating
    let delta = anim.tail_target - anim.tail_pos;

    if dt >= remaining {
        anim.tail_pos = anim.tail_target;
    } else {
        let k = dt / remaining;
        anim.tail_pos += delta * k;
    }
}
