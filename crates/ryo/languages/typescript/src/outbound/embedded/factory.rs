//! Builds the embedded adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by an edge.

use super::adapters::svelte::adapter::EmbeddedSvelteAdapter;
use super::adapters::vue::adapter::EmbeddedVueAdapter;

/// The adapter that reads the blocks of one single-file component format.
#[must_use]
pub(crate) fn vue() -> impl super::interface::EmbeddedPort {
    EmbeddedVueAdapter
}

/// The adapter that reads the blocks of another single-file component format.
#[must_use]
pub(crate) fn svelte() -> impl super::interface::EmbeddedPort {
    EmbeddedSvelteAdapter
}
