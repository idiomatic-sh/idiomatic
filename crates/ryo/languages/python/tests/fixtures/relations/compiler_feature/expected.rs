// Canonical validated module-prefix future activation is an explicit feature row.
use ryo::languages::contract::{CompilerFeatureExpectedRow, Expected, expected_rows, file, name};
const ROWS: &[CompilerFeatureExpectedRow] = &[CompilerFeatureExpectedRow {
    file: file("compiler_feature/input.py"),
    feature: name("python.future-annotations"),
    enabled: true,
}];
/// Independent expected source-language compiler mode.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
