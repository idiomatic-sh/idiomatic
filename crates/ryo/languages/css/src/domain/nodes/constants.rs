//! The names of the `nodes` concept: the standard its nodes are named by, and why the relations read none of most of them.

/// The standard the nodes are named by: its section and the version pinned in `tests/fixtures/standard/`.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str = "CSS Syntax Module Level 3, §5.2 CSS Parsing Results, W3C Candidate Recommendation Draft 1 October 2026 (https://www.w3.org/TR/2026/CRD-css-syntax-3-20261001/)";

/// A rule, a declaration or a block.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STRUCTURE: &str = "a rule, a declaration or a block: a stylesheet is no keyed document, so no relation of the catalog models it; `syntax_tree` and `syntax_child` hold it as it stands";
