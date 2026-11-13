//! Independently reviewed expected rows for every supported syntax relation.

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
/// The rows the `directive` relation must give for its fixture.
mod directive {
    include!("../../tests/fixtures/relations/directive/expected.rs");
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

/// Independently expected source coordinates for a minimal input.
mod source_location {
    include!("../../tests/fixtures/relations/source_location/expected.rs");
}
/// The rows the `visibility` relation must give for its fixture.
mod visibility {
    include!("../../tests/fixtures/relations/visibility/expected.rs");
}
/// The rows the `pattern-parse` relation must give for its fixture.
mod pattern_parse {
    include!("../../tests/fixtures/relations/pattern_parse/expected.rs");
}
/// The rows the `pattern-syntax` relation must give for its fixture.
mod pattern_syntax {
    include!("../../tests/fixtures/relations/pattern_syntax/expected.rs");
}
/// The rows the `pattern-child` relation must give for its fixture.
mod pattern_child {
    include!("../../tests/fixtures/relations/pattern_child/expected.rs");
}
/// The expected rows of every relation the language reads from trees, one entry for each.
pub(super) fn expected() -> Vec<Expected> {
    vec![
        syntax_tree::expected(),
        semantic_syntax::expected(),
        source_location::expected(),
        comment::expected(),
        string_literal::expected(),
        directive::expected(),
        function::expected(),
        scope::expected(),
        static_import::expected(),
        parameter::expected(),
        defines::expected(),
        reference::expected(),
        static_call::expected(),
        resolves::expected(),
        call_argument::expected(),
        annotates::expected(),
        assigns::expected(),
        decorates::expected(),
        visibility::expected(),
        inherits::expected(),
        syntax_child::expected(),
        semantic_child::expected(),
        syntax_error::expected(),
        pattern_parse::expected(),
        pattern_syntax::expected(),
        pattern_child::expected(),
    ]
}

/// The columns of the relations the language reads that its fixtures cannot show filled, each with why it never applies.
pub(super) fn unfilled() -> Vec<Unfilled> {
    let mut found = vec![
        Unfilled {
            relation: "directive",
            column: "text",
            unshown: Unshown::Filled,
            reason: Why(
                "an ECMAScript directive (`\"use strict\"`) is a string literal statement of a directive prologue: it takes no argument",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "condition",
            unshown: Unshown::Filled,
            reason: Why(
                "a package.json dependency applies everywhere: npm, pnpm and bun write no condition on one (`os` and `cpu` are the package's own)",
            ),
        },
    ];
    found.extend(words(
        "defines",
        "kind",
        &["struct", "trait", "type", "package", "macro", "section", "implementation"],
        "TypeScript declares a type by a class, an interface, a type alias or an enum, each its own kind, and has no struct, trait, package, macro, section or implementation declaration",
    ));
    found.extend(words(
        "defines",
        "kind",
        &["parameter"],
        "a parameter is a row of `parameter`, with its kind and place, not of `defines`",
    ));
    found.extend(words(
        "parameter",
        "kind",
        &["positional-or-keyword", "keyword-only", "variadic-keyword", "result"],
        "ECMAScript passes arguments by position only, gathers the rest by `...`, and names no result",
    ));
    found.push(Unfilled {
        relation: "package_dependency",
        column: "interpreter",
        unshown: Unshown::Filled,
        reason: Why(
            "a package.json dependency names no version of Node: `engines` is the package's own, never one dependency's",
        ),
    });
    for column in ["branch", "tag"] {
        found.push(Unfilled {
            relation: "package_dependency",
            column,
            unshown: Unshown::Filled,
            reason: Why(
                "a package.json git spec names a commit-ish after its `#` (`github:owner/repository#v1.2.0`), which npm reads as a branch, a tag or a commit without the spec saying which: it is the `revision`",
            ),
        });
    }
    found.extend(words(
        "package_dependency",
        "kind",
        &["build"],
        "package.json has `dependencies`, `devDependencies`, `peerDependencies` and `optionalDependencies`, and no build dependency",
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
