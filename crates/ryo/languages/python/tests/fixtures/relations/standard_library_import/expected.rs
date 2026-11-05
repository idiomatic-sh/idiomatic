// A dotted import keeps the real alias node and full target when its root is standard.
use ryo::languages::contract::{
    Expected, StandardLibraryImportExpectedRow, expected_rows, file, name, node,
};

const ROWS: &[StandardLibraryImportExpectedRow] = &[StandardLibraryImportExpectedRow {
    occurrence: node(4),
    target: name("os.path"),
    file: file("standard_library_import/input.py"),
}];

pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
