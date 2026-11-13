//! Canonical provider-diagnostic projection shared by real oxlint entry points.
/// Project every parse error: the TypeScript provider reports each one the parser found, with an incomplete tree, as oxlint reports
/// each one as a diagnostic without a code.
pub(crate) const FRONTEND_DIAGNOSTIC_QUERY: &str = "let diagnostics = select id, file, span from \"syntax-error\";\nemit diagnostics subject file key(id);\n";
