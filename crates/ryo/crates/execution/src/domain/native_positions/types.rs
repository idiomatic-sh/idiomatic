//! Inputs and invocation-owned scratch for native position operations.

use crate::domain::{
    lowering::types::LoweredShape,
    positions::types::{FoundPositions, Keeps, Keys, Positions, Slots, Stride},
};
use ryo_relations::domain::relations::types::Value;

/// A position input admitted and retained by the application owner.
#[derive(Clone, Copy)]
pub(crate) struct PositionInput<'a> {
    pub shape: &'a LoweredShape,
    pub rows: &'a Positions,
}

/// Scratch belongs to one engine invocation; no input or owner is retained here.
#[derive(Default)]
pub(crate) struct PositionWork {
    pub keys: Keys,
    pub probe: Keys,
    pub kept: Keeps,
    pub cells: Vec<Value>,
    pub found: FoundPositions,
    /// Union rows staged by the key phase for the evaluation phase.
    pub staged: Slots,
}

/// Row keys the owner orders into a raw order before evaluation, with their column count.
pub(crate) struct RawKeys {
    pub keys: Keys,
    pub stride: Stride,
}
