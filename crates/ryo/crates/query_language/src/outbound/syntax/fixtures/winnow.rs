//! The real parser, for the tests that read query text: the winnow adapter, composed for them.

use super::super::factory;
use super::super::interface::SyntaxPort;

/// The winnow syntax adapter, as a test composes it.
#[must_use]
pub fn winnow() -> impl SyntaxPort {
    factory::winnow()
}
