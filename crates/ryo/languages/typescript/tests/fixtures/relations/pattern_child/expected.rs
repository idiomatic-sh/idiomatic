// The rows `relations/pattern_child/input.ts` must give.

use ryo::languages::contract::{
    Expected, PatternChildExpectedRow, expected_rows, file, name, node, number,
};

/// The rows, in the order the language gives them.
const ROWS: &[PatternChildExpectedRow] = &[
    PatternChildExpectedRow { parent: node(14), child: node(15), ordinal: number(0), field: name("body"), file: file("pattern_child/input.ts") },
    PatternChildExpectedRow { parent: node(15), child: node(16), ordinal: number(0), field: name("body"), file: file("pattern_child/input.ts") },
    PatternChildExpectedRow { parent: node(15), child: node(18), ordinal: number(1), field: name("body"), file: file("pattern_child/input.ts") },
    PatternChildExpectedRow { parent: node(16), child: node(17), ordinal: number(0), field: name("body"), file: file("pattern_child/input.ts") },
    PatternChildExpectedRow { parent: node(18), child: node(19), ordinal: number(0), field: name("body"), file: file("pattern_child/input.ts") },
    PatternChildExpectedRow { parent: node(19), child: node(20), ordinal: number(0), field: name("body"), file: file("pattern_child/input.ts") },
    PatternChildExpectedRow { parent: node(20), child: node(21), ordinal: number(0), field: name("min"), file: file("pattern_child/input.ts") },
    PatternChildExpectedRow { parent: node(20), child: node(22), ordinal: number(1), field: name("max"), file: file("pattern_child/input.ts") },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
