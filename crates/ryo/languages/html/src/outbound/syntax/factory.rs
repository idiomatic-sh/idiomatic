//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `swc_html_parser` syntax adapter: the parser library the language reads with.
#[must_use]
pub(crate) fn swc_html_parser() -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::swc_html_parser::adapter::SyntaxSwcHtmlParserAdapter
}
