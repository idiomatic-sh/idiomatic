//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `toml_parser` syntax adapter: the parser of the `toml` crate.
#[must_use]
pub(crate) fn toml_parser() -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::toml_parser::adapter::SyntaxTomlParserAdapter
}
