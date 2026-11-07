//! Builds the language adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

pub use super::adapters::use_case::connection::LanguageConnection;

/// The `use_case` language adapter, over the languages the edge linked, in the order the adapter lists and numbers them in.
#[must_use]
pub fn use_case(
    languages: Vec<LanguageConnection>,
) -> impl crate::outbound::language::interface::LanguagePort {
    let mut adapter = super::adapters::use_case::adapter::LanguageUseCaseAdapter::new();
    for language in languages {
        adapter = adapter.with(language);
    }
    adapter
}
