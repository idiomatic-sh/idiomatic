//! The constants of the `flattening` part: the bounds a flattening is cut at (`swarm/lower` §2: B, P and A).

use super::types::CutBounds;

/// At most 32 branches an input, 256 a join's product and 40 atoms a branch: past one, an input is one use of a definition.
pub(crate) const CUT_BOUNDS: CutBounds = CutBounds {
    branches: 32,
    product: 256,
    atoms: 40,
};
