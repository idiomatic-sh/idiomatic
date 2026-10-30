//! The rows each relation read from trees must give for its fixture, one module for each relation.

#![expect(
    clippy::inline_modules,
    reason = "each relation's expected rows are a file of their own under tests/fixtures, which `--bless` writes; `include!` makes each one a module here, and the module spec declares no file by `#[path]` but a file's own test"
)]

use ryo::languages::Why;
use ryo::languages::contract::{Expected, Unfilled, Unshown};

/// The rows the `resolves` relation must give for its fixture input.
mod resolves {
    include!("../../tests/fixtures/relations/resolves/expected.rs");
}
/// The rows the `call_argument` relation must give for its fixture input.
mod call_argument {
    include!("../../tests/fixtures/relations/call_argument/expected.rs");
}
/// The rows the `annotates` relation must give for its fixture.
mod annotates {
    include!("../../tests/fixtures/relations/annotates/expected.rs");
}
/// The rows the `assigns` relation must give for its fixture.
mod assigns {
    include!("../../tests/fixtures/relations/assigns/expected.rs");
}
/// The rows the `comment` relation must give for its fixture.
mod comment {
    include!("../../tests/fixtures/relations/comment/expected.rs");
}
/// The rows the `decorates` relation must give for its fixture.
mod decorates {
    include!("../../tests/fixtures/relations/decorates/expected.rs");
}
/// The rows the `defines` relation must give for its fixture.
mod defines {
    include!("../../tests/fixtures/relations/defines/expected.rs");
}
/// The rows the `function` relation must give for its fixture.
mod function {
    include!("../../tests/fixtures/relations/function/expected.rs");
}
/// The rows the `inherits` relation must give for its fixture.
mod inherits {
    include!("../../tests/fixtures/relations/inherits/expected.rs");
}
/// The rows the `parameter` relation must give for its fixture.
mod parameter {
    include!("../../tests/fixtures/relations/parameter/expected.rs");
}
/// The rows the `reference` relation must give for its fixture.
mod reference {
    include!("../../tests/fixtures/relations/reference/expected.rs");
}
/// The rows the `scope` relation must give for its fixture.
mod scope {
    include!("../../tests/fixtures/relations/scope/expected.rs");
}
/// The rows the `static-call` relation must give for its fixture.
mod static_call {
    include!("../../tests/fixtures/relations/static_call/expected.rs");
}
/// The rows the `static-import` relation must give for its fixture.
mod static_import {
    include!("../../tests/fixtures/relations/static_import/expected.rs");
}
/// The rows the `string-literal` relation must give for its fixture.
mod string_literal {
    include!("../../tests/fixtures/relations/string_literal/expected.rs");
}
/// The rows the `syntax-child` relation must give for its fixture.
mod syntax_child {
    include!("../../tests/fixtures/relations/syntax_child/expected.rs");
}
/// The rows the `syntax-error` relation must give for its fixture.
mod syntax_error {
    include!("../../tests/fixtures/relations/syntax_error/expected.rs");
}
/// The rows the `syntax-tree` relation must give for its fixture.
mod syntax_tree {
    include!("../../tests/fixtures/relations/syntax_tree/expected.rs");
}

/// The expected rows of every relation the language reads from trees, one entry for each.
pub(super) fn expected() -> Vec<Expected> {
    vec![
        annotates::expected(),
        assigns::expected(),
        comment::expected(),
        decorates::expected(),
        defines::expected(),
        function::expected(),
        inherits::expected(),
        parameter::expected(),
        reference::expected(),
        scope::expected(),
        static_call::expected(),
        resolves::expected(),
        call_argument::expected(),
        static_import::expected(),
        string_literal::expected(),
        syntax_child::expected(),
        syntax_error::expected(),
        syntax_tree::expected(),
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
                "Python documents with a docstring, a string the language reads as `__doc__` (a `string_literal` row): a `#` comment is never documentation",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "catalog",
            unshown: Unshown::Filled,
            reason: Why(
                "no Python manifest takes a requirement from a shared table: PEP 621, Poetry, Pipenv and requirements files write each requirement themselves, and uv's `workspace = true` says where a package comes from (`origin`), its requirement staying in `dependencies`",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "kind",
            unshown: Unshown::Word("peer"),
            reason: Why(
                "Python packaging (PEP 508, PEP 621, PEP 735) has no peer dependency: a package states what it needs, never what its user must install beside it",
            ),
        },
    ];
    found.extend(words(
        "defines",
        "kind",
        &["struct", "enum", "enum-member", "trait", "interface", "type", "namespace", "package", "constant", "macro", "section", "label", "implementation"],
        "the Python Language Reference has no such declaration: `class` declares every type (an enum or a protocol is a class), a name is bound by assignment with no constant form, a package is a directory, and there are no macros, labels or implementations",
    ));
    found.extend(words(
        "defines",
        "kind",
        &["property"],
        "Python declares no field: a name a class body binds is a class variable, a `variable` row (Language Reference 9.3.4), and `@property` is a decorator, a `decorates` row",
    ));
    found.extend(words(
        "parameter",
        "kind",
        &["receiver", "result"],
        "a Python method's `self` is an ordinary first parameter the language does not set apart (Language Reference 8.7), and a function names no results",
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
