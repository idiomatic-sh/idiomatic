// The rows `relations/compiler_feature/input.ts` must give: TypeScript syntax.

use ryo::languages::contract::{CompilerFeatureExpectedRow, Expected, expected_rows, file, name};

/// The rows, in the order the language gives them.
const ROWS: &[CompilerFeatureExpectedRow] = &[
    CompilerFeatureExpectedRow {
        file: file("compiler_feature/input.ts"),
        feature: name("typescript.syntax"),
        enabled: true,
    },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
