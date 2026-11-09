//! Builds the observer adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The counting observer: it adds up what it is told.
#[must_use]
pub fn counting() -> impl crate::outbound::observer::interface::ObserverPort {
    super::adapters::counting::adapter::ObserverCountingAdapter::default()
}
