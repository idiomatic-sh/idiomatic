struct Service;
enum Mode { On }
trait Run {}
fn run() { let value = 1; let (a, b) = (1, 2); let _ = 3; }

use std::fmt::{self, Display as Show};
extern crate alloc;
struct Pair<'a, T, const N: usize>(&'a T, [u8; N]);
struct Point { x: i32 }
enum Shape { Dot, Line { length: u32 } }
impl Point { fn new() -> Self { Point { x: 0 } } fn x(&self) -> i32 { self.x } }
use std::io::*;

const LIMIT: u8 = 1;
static mut COUNT: u8 = 0;
mod inner { type Alias = u8; }
union Bits { a: u8 }
macro_rules! twice { () => {}; }
macro emit() {}
const _: () = ();

impl<T: Clone> Display for Point<T> where T: Copy { fn fmt() {} }
impl Point {}

fn spin() { 'outer: loop { break 'outer; } }
