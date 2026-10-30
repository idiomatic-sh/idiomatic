//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `ruff_python_parser` syntax adapter: the parser ruff and ty run.
#[must_use]
pub(crate) fn ruff_python_parser() -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::ruff_python_parser::adapter::SyntaxRuffPythonParserAdapter
}
