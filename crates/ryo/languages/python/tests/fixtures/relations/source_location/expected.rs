// Reuses the independently pinned syntax_child/input.py bytes. Coordinates are derived from those bytes and the pre-change tree reader.

use ryo::languages::contract::{
    Expected, SourceLocationExpectedRow, expected_rows, file, node, number, span,
};

const ROWS: &[SourceLocationExpectedRow] = &[
    SourceLocationExpectedRow {
        id: node(0),
        file: file("source_location/input.py"),
        span: span((1, 1), (3, 1)),
        start_line: number(1),
        end_line: number(3),
    },
    SourceLocationExpectedRow {
        id: node(1),
        file: file("source_location/input.py"),
        span: span((1, 1), (1, 10)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(2),
        file: file("source_location/input.py"),
        span: span((1, 1), (1, 7)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(3),
        file: file("source_location/input.py"),
        span: span((1, 7), (1, 8)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(4),
        file: file("source_location/input.py"),
        span: span((1, 8), (1, 10)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(5),
        file: file("source_location/input.py"),
        span: span((1, 8), (1, 10)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(6),
        file: file("source_location/input.py"),
        span: span((1, 8), (1, 10)),
        start_line: number(1),
        end_line: number(1),
    },
    SourceLocationExpectedRow {
        id: node(7),
        file: file("source_location/input.py"),
        span: span((1, 10), (2, 1)),
        start_line: number(1),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(8),
        file: file("source_location/input.py"),
        span: span((2, 1), (2, 14)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(9),
        file: file("source_location/input.py"),
        span: span((2, 1), (2, 14)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(10),
        file: file("source_location/input.py"),
        span: span((2, 1), (2, 6)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(11),
        file: file("source_location/input.py"),
        span: span((2, 1), (2, 6)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(12),
        file: file("source_location/input.py"),
        span: span((2, 6), (2, 14)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(13),
        file: file("source_location/input.py"),
        span: span((2, 6), (2, 7)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(14),
        file: file("source_location/input.py"),
        span: span((2, 7), (2, 13)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(15),
        file: file("source_location/input.py"),
        span: span((2, 7), (2, 9)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(16),
        file: file("source_location/input.py"),
        span: span((2, 7), (2, 9)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(17),
        file: file("source_location/input.py"),
        span: span((2, 9), (2, 10)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(18),
        file: file("source_location/input.py"),
        span: span((2, 10), (2, 13)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(19),
        file: file("source_location/input.py"),
        span: span((2, 10), (2, 13)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(20),
        file: file("source_location/input.py"),
        span: span((2, 13), (2, 14)),
        start_line: number(2),
        end_line: number(2),
    },
    SourceLocationExpectedRow {
        id: node(21),
        file: file("source_location/input.py"),
        span: span((2, 14), (3, 1)),
        start_line: number(2),
        end_line: number(3),
    },
];

/// The independently expected source coordinates.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
