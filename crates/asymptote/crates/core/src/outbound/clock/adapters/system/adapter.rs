//! The clock port, implemented with jiff: today in the machine's time zone.

use crate::outbound::clock::interface::ClockPort;
use asymptote_values::domain::operations::types::Day;

/// The system clock adapter.
#[derive(Debug)]
pub(crate) struct ClockSystemAdapter;

impl ClockPort for ClockSystemAdapter {
    fn today(&self) -> Day {
        Day(jiff::Zoned::now().date().to_string().into())
    }
}
