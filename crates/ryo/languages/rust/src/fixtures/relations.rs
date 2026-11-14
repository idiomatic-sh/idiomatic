//! The rows each relation read from trees must give for its fixture, one module for each relation.

#![expect(
    clippy::inline_modules,
    reason = "each relation's expected rows are a file of their own under tests/fixtures, which `--bless` writes; `include!` makes each one a module here, and the module spec declares no file by `#[path]` but a file's own test"
)]

use ryo::languages::Why;
use ryo::languages::contract::{Expected, Unfilled, Unshown};

/// The rows `annotates` read from its fixture's input must give.
mod annotates {
    include!("../../tests/fixtures/relations/annotates/expected.rs");
}
/// The rows `assigns` read from its fixture's input must give.
mod assigns {
    include!("../../tests/fixtures/relations/assigns/expected.rs");
}
/// The rows `call_argument` read from its fixture's input must give.
mod call_argument {
    include!("../../tests/fixtures/relations/call_argument/expected.rs");
}
/// The rows `comment` read from its fixture's input must give.
mod comment {
    include!("../../tests/fixtures/relations/comment/expected.rs");
}
/// The rows `decorates` read from its fixture's input must give.
mod decorates {
    include!("../../tests/fixtures/relations/decorates/expected.rs");
}
/// The rows `defines` read from its fixture's input must give.
mod defines {
    include!("../../tests/fixtures/relations/defines/expected.rs");
}
/// The rows `expression_shape` read from its fixture's input must give.
mod expression_shape {
    include!("../../tests/fixtures/relations/expression_shape/expected.rs");
}
/// The rows `function` read from its fixture's input must give.
mod function {
    include!("../../tests/fixtures/relations/function/expected.rs");
}
/// The rows `inherits` read from its fixture's input must give.
mod inherits {
    include!("../../tests/fixtures/relations/inherits/expected.rs");
}
/// The rows `mention` read from its fixture's input must give.
mod mention {
    include!("../../tests/fixtures/relations/mention/expected.rs");
}
/// The rows `numeric_literal` read from its fixture's input must give.
mod numeric_literal {
    include!("../../tests/fixtures/relations/numeric_literal/expected.rs");
}
/// The rows `parameter` read from its fixture's input must give.
mod parameter {
    include!("../../tests/fixtures/relations/parameter/expected.rs");
}
/// The rows `reference` read from its fixture's input must give.
mod reference {
    include!("../../tests/fixtures/relations/reference/expected.rs");
}
/// The rows `resolves` read from its fixture's input must give.
mod resolves {
    include!("../../tests/fixtures/relations/resolves/expected.rs");
}
/// The rows `scope` read from its fixture's input must give.
mod scope {
    include!("../../tests/fixtures/relations/scope/expected.rs");
}
/// The rows `static_call` read from its fixture's input must give.
mod static_call {
    include!("../../tests/fixtures/relations/static_call/expected.rs");
}
/// The rows `static_import` read from its fixture's input must give.
mod static_import {
    include!("../../tests/fixtures/relations/static_import/expected.rs");
}
/// The rows `string_literal` read from its fixture's input must give.
mod string_literal {
    include!("../../tests/fixtures/relations/string_literal/expected.rs");
}
/// The rows `syntax_child` read from its fixture's input must give.
mod syntax_child {
    include!("../../tests/fixtures/relations/syntax_child/expected.rs");
}
/// The rows `syntax_error` read from its fixture's input must give.
mod syntax_error {
    include!("../../tests/fixtures/relations/syntax_error/expected.rs");
}
/// The rows `syntax_tree` read from its fixture's input must give.
mod syntax_tree {
    include!("../../tests/fixtures/relations/syntax_tree/expected.rs");
}

/// Independently expected source coordinates for a minimal input.
mod source_location {
    include!("../../tests/fixtures/relations/source_location/expected.rs");
}
/// The rows `visibility` read from its fixture's input must give.
mod visibility {
    include!("../../tests/fixtures/relations/visibility/expected.rs");
}

/// The rows of every relation.
pub(super) fn expected() -> Vec<Expected> {
    vec![
        annotates::expected(),
        assigns::expected(),
        call_argument::expected(),
        comment::expected(),
        decorates::expected(),
        defines::expected(),
        expression_shape::expected(),
        function::expected(),
        inherits::expected(),
        mention::expected(),
        numeric_literal::expected(),
        parameter::expected(),
        reference::expected(),
        resolves::expected(),
        scope::expected(),
        static_call::expected(),
        static_import::expected(),
        string_literal::expected(),
        syntax_child::expected(),
        semantic_child::expected(),
        syntax_error::expected(),
        syntax_tree::expected(),
        semantic_syntax::expected(),
        source_location::expected(),
        visibility::expected(),
    ]
}

/// The columns of the relations the language reads that its fixtures cannot show filled, each with why it never applies.
pub(super) fn unfilled() -> Vec<Unfilled> {
    let mut found = vec![
        Unfilled {
            relation: "defines",
            column: "kind",
            unshown: Unshown::Word("parameter"),
            reason: Why("a function's parameters are the rows of `parameter`, not of `defines`"),
        },
        Unfilled {
            relation: "package_dependency",
            column: "interpreter",
            unshown: Unshown::Filled,
            reason: Why(
                "a Cargo dependency names no version of Rust: `rust-version` is the package's own (the Cargo Book, \"The `rust-version` field\")",
            ),
        },
    ];
    found.extend(words(
        "defines",
        "kind",
        &["class", "interface", "type", "package", "section"],
        "the Rust Reference has no class, interface, package or section item, and every type item is a struct, an enum, a union or an alias, which have their own kinds",
    ));
    found.extend(words(
        "parameter",
        "kind",
        &["positional-or-keyword", "keyword-only", "variadic-keyword"],
        "Rust passes every argument by position: the Rust Reference has no keyword argument",
    ));
    found.extend(words(
        "parameter",
        "kind",
        &["result"],
        "a Rust function's result is a type, never a named parameter",
    ));
    found.extend(words(
        "package_dependency",
        "kind",
        &["peer"],
        "Cargo has no peer dependency",
    ));
    found.extend(words(
        "package_dependency",
        "origin",
        &["url", "workspace"],
        "Cargo takes a package from a registry, a `path` or a `git` repository only; `workspace = true` inherits the origin of the workspace's own entry, and a member is reached by its `path`",
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
