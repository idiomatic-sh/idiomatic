//! Instrumentation protocol and refusal vocabulary.
pub(super) const PIN_MISMATCH: &str = "instrumentation executable differs from pin";
pub(super) const SIGNAL: &str = "instrumentation terminated by signal";
pub(super) const MISSING_CHILD: &str = "contestant executable not found";
pub(super) const RESOURCE_QUIET: &str = "--quiet";
pub(super) const FORMAT_FLAG: &str = "--format";
pub(super) const FORMAT: &str = "%U\t%S\t%M";
pub(super) const OUTPUT: &str = "--output";
pub(super) const SEPARATOR: &str = "--";
pub(super) const LOCALE: &str = "LC_ALL";
pub(super) const PORTABLE_LOCALE: &str = "C";
pub(super) const PATH: &str = "PATH";
