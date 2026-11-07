//! Builds the matching adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The adapter that answers the port with the matching crate's use case.
#[must_use]
pub fn use_case() -> impl super::interface::MatchingPort {
    super::adapters::use_case::adapter::MatchingUseCaseAdapter::new(
        ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new(),
    )
}
