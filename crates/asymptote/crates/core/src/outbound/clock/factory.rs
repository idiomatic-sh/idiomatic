//! Builds the clock adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge.

/// The system clock, read in the machine's time zone.
#[must_use]
pub fn system() -> impl crate::outbound::clock::interface::ClockPort {
    super::adapters::system::adapter::ClockSystemAdapter
}
