// The rows `relations/static_import/input.rs` must give.

use ryo::languages::contract::{
    Expected, StaticImportExpectedRow, expected_rows, file, name, node, span,
};

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(&[
        StaticImportExpectedRow {
            occurrence: node(20),
            owner: node(0),
            binding: name("io"),
            target: name("std::io"),
            file: file("static_import/input.rs"),
            span: span((1, 16), (1, 20)),
        },
        StaticImportExpectedRow {
            occurrence: node(27),
            owner: node(0),
            binding: name("Write"),
            target: name("std::io::Write"),
            file: file("static_import/input.rs"),
            span: span((1, 22), (1, 27)),
        },
        StaticImportExpectedRow {
            occurrence: node(35),
            owner: node(0),
            binding: name("*"),
            target: name("std::fmt::*"),
            file: file("static_import/input.rs"),
            span: span((1, 30), (1, 36)),
        },
        StaticImportExpectedRow {
            occurrence: node(48),
            owner: node(0),
            binding: name("svc"),
            target: name("crate::service"),
            file: file("static_import/input.rs"),
            span: span((3, 5), (3, 26)),
        },
    ])
}
