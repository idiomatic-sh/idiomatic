//! Canonical provider-diagnostic projection shared by real Ruff entry points.
/// Project grammar errors and syntax validation that has no dedicated rule.
/// Future directives parse into a complete AST and retain their `syntax-error`
/// facts for feature gating, while F404/F407 consume their directive validation
/// diagnostics. A prohibited lazy future statement has a separate cause and
/// remains a generic validity finding regardless of selected lint rules.
/// Other validation errors, such as invalid `nonlocal` and
/// annotation expressions, continue to project as generic invalid syntax.
pub(crate) const FRONTEND_DIAGNOSTIC_QUERY: &str = "let diagnostics = select id, file, span from \"syntax-error\" where (phase = \"parse\" and (impact = \"incomplete\" or impact = \"lexically-complete\" or impact = \"preserved\")) or (phase = \"validation\" and impact = \"preserved\" and code != \"future-feature-not-defined\" and code != \"future-directive-invalid\");\nemit diagnostics subject file key(id);\n";
