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
/// The rows the `entry` relation must give for its fixture input.
mod entry {
    include!("../../tests/fixtures/relations/entry/expected.rs");
}
/// The rows the `mapping` relation must give for its fixture input.
mod mapping {
    include!("../../tests/fixtures/relations/mapping/expected.rs");
}
/// The rows the `sequence_element` relation must give for its fixture input.
mod sequence_element {
    include!("../../tests/fixtures/relations/sequence_element/expected.rs");
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
        entry::expected(),
        mapping::expected(),
        sequence_element::expected(),
        string_literal::expected(),
        syntax_child::expected(),
        syntax_error::expected(),
        syntax_tree::expected(),
        source_location::expected(),
    ]
}

/// The columns of the relations the language reads that its fixtures cannot show filled, each with why it never applies.
pub(super) fn unfilled() -> Vec<Unfilled> {
    let mut found = vec![
        Unfilled {
            relation: "comment",
            column: "doc",
            unshown: Unshown::Word("true"),
            reason: Why(
                "TOML has one kind of comment, `#`, and no convention that makes one documentation",
            ),
        },
        Unfilled {
            relation: "string_literal",
            column: "import",
            unshown: Unshown::Word("true"),
            reason: Why("a TOML document loads no code: no string of it is an import specifier"),
        },
        Unfilled {
            relation: "entry",
            column: "kind",
            unshown: Unshown::Word("null"),
            reason: Why("TOML has no value that says there is none"),
        },
    ];
    found.extend(words(
        "sequence_element",
        "kind",
        &["sequence-of-mappings", "null"],
        "an element of a TOML array is a value or a table: no element opens an array of tables, and TOML has no null",
    ));
    found
}

/// One declaration for each word of a closed set the language never writes, all for one reason.
fn words(
    relation: &'static str,
    column: &'static str,
    words: &'static [&'static str],
    reason: &'static str,
) -> impl Iterator<Item = Unfilled> {
    words.iter().map(move |word| Unfilled {
        relation,
        column,
        unshown: Unshown::Word(word),
        reason: Why(reason),
    })
}
