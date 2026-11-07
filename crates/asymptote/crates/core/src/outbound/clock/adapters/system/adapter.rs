//! The clock port, implemented with jiff: today in the machine's time zone.

use crate::domain::operations::types::Day;
use crate::outbound::clock::interface::ClockPort;

/// The system clock adapter.
#[derive(Debug)]
pub(crate) struct ClockSystemAdapter;

impl ClockPort for ClockSystemAdapter {
    fn today(&self) -> Day {
        Day(jiff::Zoned::now().date().to_string().into())
    }
}
