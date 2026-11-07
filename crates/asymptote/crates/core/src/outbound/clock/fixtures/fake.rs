//! The fake of the `clock` port: one fixed day.

use crate::domain::operations::types::Day;
use crate::outbound::clock::interface::ClockPort;

/// What the core's tests stand in for `clock` with: always 2026-10-19.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClockFake;

impl ClockPort for ClockFake {
    fn today(&self) -> Day {
        Day("2026-10-19".into())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
