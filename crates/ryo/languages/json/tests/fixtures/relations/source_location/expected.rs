// Reuses the independently pinned syntax_child/input.json bytes. Coordinates are derived from those bytes and the pre-change tree reader.

use ryo::languages::contract::{
    Expected, SourceLocationExpectedRow, expected_rows, file, node, number, span,
};

const ROWS: &[SourceLocationExpectedRow] = &[
    SourceLocationExpectedRow {
        id: node(0),
        file: file("source_location/input.json"),
        span: span((1, 1), (2, 1)),
        start_line: number(1),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(1),
        file: file("source_location/input.json"),
        span: span((1, 1), (1, 11)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(2),
        file: file("source_location/input.json"),
        span: span((1, 1), (1, 2)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(3),
        file: file("source_location/input.json"),
        span: span((1, 2), (1, 10)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(4),
        file: file("source_location/input.json"),
        span: span((1, 2), (1, 5)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(5),
        file: file("source_location/input.json"),
        span: span((1, 5), (1, 6)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(6),
        file: file("source_location/input.json"),
        span: span((1, 6), (1, 7)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(7),
        file: file("source_location/input.json"),
        span: span((1, 7), (1, 10)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(8),
        file: file("source_location/input.json"),
        span: span((1, 7), (1, 8)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(9),
        file: file("source_location/input.json"),
        span: span((1, 8), (1, 9)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(10),
        file: file("source_location/input.json"),
        span: span((1, 9), (1, 10)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(11),
        file: file("source_location/input.json"),
        span: span((1, 10), (1, 11)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(12),
        file: file("source_location/input.json"),
        span: span((1, 11), (2, 1)),
        start_line: number(1),
        end_line: number(2),
    },
];

/// The independently expected source coordinates.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
