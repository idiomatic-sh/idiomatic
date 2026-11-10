//! Stable partial-input cause owned by this format concept.
/// Why brace-format rows are partial.
pub(crate) const INCOMPLETE: &str = "brace-format parsing needs complete decoded string inputs";
/// In-process brace memo mode; parser implementation identity is checked separately.
pub(crate) const GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
    crate::domain::format_syntax::types::GrammarIdentity("brace-decoded");
