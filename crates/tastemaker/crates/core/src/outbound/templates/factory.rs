//! Builds the templates adapters, one function each. A factory builds nothing else, chooses nothing, and is called only
//! by an edge.

/// The minijinja adapter, with the case filters of heck.
#[must_use]
pub fn minijinja() -> impl crate::outbound::templates::interface::TemplatesPort {
    super::adapters::minijinja::adapter::TemplatesMinijinjaAdapter::new()
}
