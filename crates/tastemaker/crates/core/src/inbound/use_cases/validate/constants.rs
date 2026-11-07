//! The constants of the `validate` use case: how many entries each of the caches a run keeps may hold.

/// The most definitions a run keeps parsed; past it a definition is read again where it is asked for.
pub(crate) const PARSES: usize = 65_536;

/// The most package manifests a run keeps read.
pub(crate) const MANIFESTS: usize = 16_384;

/// The most lists of versions a run keeps.
pub(crate) const OFFERS: usize = 65_536;

/// The most verdicts of ryo on queries a run keeps.
pub(crate) const VERDICTS: usize = 65_536;

/// The most wrapper packages a run keeps made.
pub(crate) const WRAPPERS: usize = 16_384;
