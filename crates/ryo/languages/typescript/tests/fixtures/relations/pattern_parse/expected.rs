// The rows `relations/pattern_parse/input.ts` must give.

use ryo::languages::contract::{
    Expected, PatternParseExpectedRow, expected_rows, file, name, node, span,
};

/// The rows, in the order the language gives them.
const ROWS: &[PatternParseExpectedRow] = &[
    PatternParseExpectedRow { carrier: node(10), flags: name("gu"), outcome: name("parsed"), error: name(""), file: file("pattern_parse/input.ts"), span: span((1, 11), (1, 17)) },
    PatternParseExpectedRow { carrier: node(29), flags: name(""), outcome: name("malformed"), error: name("Invalid regular expression: Unterminated character class"), file: file("pattern_parse/input.ts"), span: span((2, 22), (2, 25)) },
    PatternParseExpectedRow { carrier: node(47), flags: name("unknown"), outcome: name("parsed"), error: name(""), file: file("pattern_parse/input.ts"), span: span((3, 18), (3, 21)) },
    PatternParseExpectedRow { carrier: node(69), flags: name("y"), outcome: name("parsed"), error: name(""), file: file("pattern_parse/input.ts"), span: span((4, 18), (4, 21)) },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
