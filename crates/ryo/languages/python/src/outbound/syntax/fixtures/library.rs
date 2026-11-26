//! The library of the `ruff_python_parser` adapter, as the tests read it: every Python source of the language's fixtures.

/// Every Python source under the language's fixtures, with its path under them.
///
/// # Errors
///
/// When a fixture folder or file cannot be read.
#[cfg(test)]
pub(crate) fn fixture_sources() -> std::io::Result<Vec<(String, Vec<u8>)>> {
    crate::outbound::syntax::adapters::ruff_python_parser::fixtures::sources::fixture_sources()
}
