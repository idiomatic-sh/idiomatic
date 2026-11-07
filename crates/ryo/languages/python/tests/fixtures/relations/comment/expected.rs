// The rows `relations/comment/input.py` must give.

use ryo::languages::contract::{
    CommentExpectedRow, Expected, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        CommentExpectedRow {
            id: node(1),
            text: name("!/usr/bin/env python3"),
            file: file("comment/input.py"),
            span: span((1, 1), (1, 23)),
            doc: false,
        },
        CommentExpectedRow {
            id: node(3),
            text: name(" -*- coding: utf-8 -*-"),
            file: file("comment/input.py"),
            span: span((2, 1), (2, 24)),
            doc: false,
        },
        CommentExpectedRow {
            id: node(14),
            text: name(" trailing"),
            file: file("comment/input.py"),
            span: span((3, 12), (3, 22)),
            doc: false,
        },
        CommentExpectedRow {
            id: node(39),
            text: name(" inside a function"),
            file: file("comment/input.py"),
            span: span((8, 5), (8, 24)),
            doc: false,
        },
    ])
}
