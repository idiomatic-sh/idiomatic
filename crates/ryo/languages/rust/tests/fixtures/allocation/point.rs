//! A file.

use std::fmt;

pub struct Point {
    pub x: u32,
    pub y: u32,
}

impl Point {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    fn sum(&self) -> u32 {
        self.x + self.y
    }
}

fn main() {
    let point = Point::new(1, 2);
    if point.sum() > 2 {
        println!("{}", point.sum());
    }
}
