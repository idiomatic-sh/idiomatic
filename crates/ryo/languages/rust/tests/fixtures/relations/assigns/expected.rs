// The rows `relations/assigns/input.rs` must give.

use ryo::languages::contract::{AssignsExpectedRow, Expected, expected_rows, file, node, span};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[AssignsExpectedRow {
        occurrence: node(58),
        binding: node(53),
        value: node(58),
        owner: node(36),
        file: file("assigns/input.rs"),
        span: span((3, 24), (3, 31)),
    }])
}
