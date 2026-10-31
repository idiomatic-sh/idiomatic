//! The rows each relation read from trees must give for its fixture, one module for each relation.

#![expect(
    clippy::inline_modules,
    reason = "each relation's expected rows are a file of their own under tests/fixtures, which `--bless` writes; `include!` makes each one a module here, and the module spec declares no file by `#[path]` but a file's own test"
)]

use ryo::languages::Why;
use ryo::languages::contract::{Expected, Unfilled, Unshown};

/// The rows the `comment` relation must give for its fixture input.
mod comment {
    include!("../../tests/fixtures/relations/comment/expected.rs");
}
/// The rows the `reference` relation must give for its fixture input.
mod reference {
    include!("../../tests/fixtures/relations/reference/expected.rs");
}
/// The rows the `static_import` relation must give for its fixture input.
mod static_import {
    include!("../../tests/fixtures/relations/static_import/expected.rs");
}
/// The rows the `string_literal` relation must give for its fixture input.
mod string_literal {
    include!("../../tests/fixtures/relations/string_literal/expected.rs");
}
/// The rows the `syntax_child` relation must give for its fixture input.
mod syntax_child {
    include!("../../tests/fixtures/relations/syntax_child/expected.rs");
}
/// The rows the `syntax_error` relation must give for its fixture input.
mod syntax_error {
    include!("../../tests/fixtures/relations/syntax_error/expected.rs");
}
/// The rows the `syntax_tree` relation must give for its fixture input.
mod syntax_tree {
    include!("../../tests/fixtures/relations/syntax_tree/expected.rs");
}

/// Independently expected source coordinates for a minimal input.
mod source_location {
    include!("../../tests/fixtures/relations/source_location/expected.rs");
}

/// The rows of every relation.
pub(super) fn expected() -> Vec<Expected> {
    vec![
        comment::expected(),
        reference::expected(),
        static_import::expected(),
        string_literal::expected(),
        syntax_child::expected(),
        syntax_error::expected(),
        syntax_tree::expected(),
        source_location::expected(),
    ]
}

/// The columns of the relations the language reads that its fixtures cannot show filled, each with why it never applies.
pub(super) fn unfilled() -> Vec<Unfilled> {
    vec![
        Unfilled {
            relation: "comment",
            column: "doc",
            unshown: Unshown::Word("true"),
            reason: Why(
                "CSS Syntax 3 has one kind of comment, `/* */`, and no convention that makes one documentation",
            ),
        },
        Unfilled {
            relation: "static_import",
            column: "binding",
            unshown: Unshown::Filled,
            reason: Why(
                "an `@import` binds no name: CSS Cascade 5 imports a style sheet's rules into the importing one, with nothing named",
            ),
        },
    ]
}
