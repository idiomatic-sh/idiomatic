//! False is an explicit typed flag, distinct from an absent relation capability.
use super::super::interface::RelationsItem as _;
use super::item::{CompilerFeature, CompilerFeatureRow};
use crate::domain::relations::types::{Bits, FileId, Flag};
use crate::domain::terms::types::Symbol;
#[test]
fn feature_flags_are_independent_of_source_and_feature_identity() {
    for (enabled, bit) in [(Flag::Raised, Bits(1)), (Flag::Lowered, Bits(0))] {
        assert_eq!(
            CompilerFeature::words(CompilerFeatureRow {
                file: FileId(7),
                feature: Symbol(19),
                enabled
            }),
            [Bits(7), Bits(19), bit]
        );
    }
}
