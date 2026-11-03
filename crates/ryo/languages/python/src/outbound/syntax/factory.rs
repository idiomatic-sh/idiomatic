//! Builds the syntax adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The `ruff_python_parser` syntax adapter: the parser ruff and ty run.
#[must_use]
pub(crate) fn ruff_python_parser() -> impl crate::outbound::syntax::interface::SyntaxPort {
    super::adapters::ruff_python_parser::adapter::SyntaxRuffPythonParserAdapter::default()
}

/// Observe actual carrier parsing on this adapter instance, without a process-global counter.
#[cfg(test)]
pub(crate) fn observed_ruff_python_parser() -> (
    impl crate::outbound::syntax::interface::SyntaxPort,
    std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    let adapter =
        super::adapters::ruff_python_parser::adapter::SyntaxRuffPythonParserAdapter::default();
    let count = std::sync::Arc::clone(&adapter.expansion_parses);
    (adapter, count)
}
