//! Canonical provider-diagnostic projection shared by real clippy entry points.
/// Project every parse error: the Rust provider reports each one the parser found, with an incomplete tree, as clippy reports
/// each one as a diagnostic without a code.
pub(crate) const FRONTEND_DIAGNOSTIC_QUERY: &str = "let diagnostics = select id, file, span from \"syntax-error\";\nemit diagnostics subject file key(id);\n";
