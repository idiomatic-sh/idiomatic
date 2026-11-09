//! Builds the observer adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

use super::adapters::counting::adapter::CountingObserver;

/// The counting observer, sized by the number of steps of each rule, in rule order.
#[must_use]
pub fn counting(steps: &[usize]) -> CountingObserver {
    CountingObserver::new(steps)
}
