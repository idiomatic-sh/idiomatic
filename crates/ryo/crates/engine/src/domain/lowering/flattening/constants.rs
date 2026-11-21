//! The constants of the `flattening` part: the bounds a flattening is cut at (`swarm/lower` §2: B, P and A). Past one, an
//! input becomes one use of a definition: a cut point, never a refusal.

/// The branches of one input: at most 32.
pub(crate) const INPUT_BRANCHES: usize = 32;
/// The branches of a join's product: at most 256.
pub(crate) const PRODUCT_BRANCHES: usize = 256;
/// The atoms of one branch: at most 40.
pub(crate) const BRANCH_ATOMS: usize = 40;
