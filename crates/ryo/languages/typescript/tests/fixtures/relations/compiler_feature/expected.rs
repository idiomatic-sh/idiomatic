// The rows `relations/compiler_feature/input.{ts,js,mjs}` must give: TypeScript syntax in the `.ts` file alone.

use ryo::languages::contract::{CompilerFeatureExpectedRow, Expected, expected_rows, file, name};

/// The rows, in the order the language gives them.
const ROWS: &[CompilerFeatureExpectedRow] = &[
    CompilerFeatureExpectedRow {
        file: file("compiler_feature/input.js"),
        feature: name("typescript.syntax"),
        enabled: false,
    },
    CompilerFeatureExpectedRow {
        file: file("compiler_feature/input.mjs"),
        feature: name("typescript.syntax"),
        enabled: false,
    },
    CompilerFeatureExpectedRow {
        file: file("compiler_feature/input.ts"),
        feature: name("typescript.syntax"),
        enabled: true,
    },
];

/// The rows the fixture's inputs must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
