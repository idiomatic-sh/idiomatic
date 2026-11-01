//! The real file identity and interned path occupy separate columns.
use super::super::interface::RelationsItem as _;
use super::item::{ModuleIdentity, ModuleIdentityRow};
use crate::domain::{
    relations::types::{Bits, FileId},
    terms::types::Symbol,
};
#[test]
fn row_words_preserve_column_order() {
    assert_eq!(
        ModuleIdentity::words(ModuleIdentityRow {
            file: FileId(3),
            path: Symbol(5)
        }),
        [Bits(3), Bits(5)]
    );
}
