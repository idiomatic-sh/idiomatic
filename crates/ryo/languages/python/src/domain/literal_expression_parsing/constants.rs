/// The text `literal-expression parsing requires complete logical string `.
pub(crate) const INCOMPLETE: &str =
    "literal-expression parsing requires complete logical string carriers";
/// The text `(\n`.
pub(crate) const CARRIER_OPEN: &str = "(\n";
/// The text `parsed`.
pub(crate) const PARSED: &str = "parsed";
/// The text `malformed`.
pub(crate) const MALFORMED: &str = "malformed";
/// The text `unavailable`.
pub(crate) const UNAVAILABLE: &str = "unavailable";
/// The outcomes of a literal parse, as a filler writes them: the same in every file, so fixed names of the run.
pub(crate) const FIXED_NAMES: &[&str] = &[PARSED, MALFORMED, UNAVAILABLE];
