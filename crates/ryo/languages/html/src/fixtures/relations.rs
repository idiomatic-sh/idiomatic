//! Every supported relation is held to full typed fixture rows.

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
/// The rows the `defines` relation must give for its fixture input.
mod defines {
    include!("../../tests/fixtures/relations/defines/expected.rs");
}
/// The rows the `prose` relation must give for its fixture input.
mod prose {
    include!("../../tests/fixtures/relations/prose/expected.rs");
}
/// The rows the `reference` relation must give for its fixture input.
mod reference {
    include!("../../tests/fixtures/relations/reference/expected.rs");
}
/// The rows the `scope` relation must give for its fixture input.
mod scope {
    include!("../../tests/fixtures/relations/scope/expected.rs");
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
/// The rows of every relation, one entry for each.
pub(super) fn expected() -> Vec<Expected> {
    vec![
        syntax_tree::expected(),
        semantic_syntax::expected(),
        source_location::expected(),
        syntax_child::expected(),
        semantic_child::expected(),
        syntax_error::expected(),
        comment::expected(),
        string_literal::expected(),
        prose::expected(),
        defines::expected(),
        reference::expected(),
        scope::expected(),
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
                "HTML has one kind of comment, `<!-- -->`, and no convention that makes one documentation",
            ),
        },
        Unfilled {
            relation: "string_literal",
            column: "import",
            unshown: Unshown::Word("true"),
            reason: Why(
                "an HTML attribute value is no import specifier: a URL an element loads (`src`, `href`) is a `reference`",
            ),
        },
    ];
    found.extend(words(
        "defines",
        "kind",
        &["function", "method", "class", "struct", "enum", "enum-member", "trait", "interface", "type", "type-alias", "type-parameter", "namespace", "package", "import", "variable", "constant", "parameter", "property", "macro", "implementation"],
        "an HTML document defines only its titled sections and the anchors its `id` and `name` attributes name, which are labels",
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

/// Native syntax is preserved by the semantic projection.
mod semantic_syntax {
    include!("../../tests/fixtures/relations/semantic_syntax/expected.rs");
}

/// Native syntax is preserved by the semantic projection.
mod semantic_child {
    include!("../../tests/fixtures/relations/semantic_child/expected.rs");
}
