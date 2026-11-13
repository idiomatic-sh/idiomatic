//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `ruff_python_parser` syntax adapter: the parser ruff and ty run, reading under the settings the edge injects.
#[must_use]
pub(crate) fn ruff_python_parser(
    settings: crate::settings::PythonParserSettings,
) -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::ruff_python_parser::adapter::SyntaxRuffPythonParserAdapter {
        settings,
        ..Default::default()
    }
}
