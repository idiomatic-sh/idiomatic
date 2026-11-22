//! The limits of the `splicing` concept.

/// How many lowered expansions one shelf keeps: past it, an expansion not kept is lowered in each plan and not kept, so a
/// session compiling ever new fragment values stays bounded.
pub(crate) const LOWERED_EXPANSIONS: usize = 4096;

/// How many environments the lowered expansions are kept over: past it, a new environment's expansions are lowered in each
/// plan and not kept.
pub(crate) const LOWERING_ENVIRONMENTS: usize = 8;
