//! Public schema and typed row words of an engine-owned input fact.
use super::super::interface::RelationsItem as _;
use super::{
    constants,
    item::{FilesystemExecutable, FilesystemExecutableColumn, FilesystemExecutableRow},
};
use crate::domain::relations::types::{Bits, FileId, Flag};
#[test]
fn schema_and_rows_preserve_file_then_boolean_without_an_unknown_sentinel() {
    assert_eq!(constants::name(), "filesystem.executable");
    assert_eq!(
        constants::column_name(FilesystemExecutableColumn::Path),
        "path"
    );
    assert_eq!(
        constants::column_name(FilesystemExecutableColumn::Executable),
        "executable"
    );
    assert_eq!(
        FilesystemExecutable::words(FilesystemExecutableRow {
            path: FileId(7),
            executable: Flag::Lowered
        }),
        [Bits(7), Bits(0)]
    );
    assert_eq!(
        FilesystemExecutable::words(FilesystemExecutableRow {
            path: FileId(7),
            executable: Flag::Raised
        }),
        [Bits(7), Bits(1)]
    );
}
