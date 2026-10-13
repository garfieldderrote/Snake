use std::ops::{Add, AddAssign, Mul, Sub};

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PlayerId(pub u64);

#[derive(Clone, Copy, PartialEq)]
pub struct IVec2 {
    pub x: i32,
    pub y: i32,
}

impl IVec2 {
    pub fn new(x: i32, y: i32) -> IVec2 {
        IVec2 { x, y }
    }
    pub fn add(&self, other: &IVec2) -> IVec2 {
        IVec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
    pub fn scale(&self, scalar: i32) -> IVec2 {
        IVec2 {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn perp_dot(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }
    pub fn dot(self, other: Vec2) -> f64 {
        self.x * other.x + self.y * other.y
    }
    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }

    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn normalize(self) -> Option<Vec2> {
        let len = self.length();
        if len <= 1e-12 {
            None
        } else {
            Some(self * (1.0 / len))
        }
    }
    pub fn lerp(self, rhs: Self, t: f64) -> Self {
        Vec2 {
            x: self.x + (rhs.x - self.x) * t,
            y: self.y + (rhs.y - self.y) * t,
        }
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Mul<i32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self::Output {
        Self {
            x: self.x * rhs as f64,
            y: self.y * rhs as f64,
        }
    }
}

impl Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, PartialEq)]
pub enum Cell {
    Empty,
    Apple,
    Snake(i32, PlayerId),
}
