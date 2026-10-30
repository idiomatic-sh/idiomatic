//! Complete expected rows for each supported JavaScript relation.

#![expect(
    clippy::inline_modules,
    reason = "each relation's expected rows are a file of their own under tests/fixtures, which `--bless` writes; `include!` makes each one a module here, and the module spec declares no file by `#[path]` but a file's own test"
)]
use ryo::languages::Why;
use ryo::languages::contract::{Expected, Unfilled, Unshown};
/// The rows the `call_argument` relation must give for its fixture input.
mod call_argument {
    include!("../../tests/fixtures/relations/call_argument/expected.rs");
}
/// The rows the `assigns` relation must give for its fixture.
mod assigns {
    include!("../../tests/fixtures/relations/assigns/expected.rs");
}
/// The rows the `defines` relation must give for its fixture.
mod defines {
    include!("../../tests/fixtures/relations/defines/expected.rs");
}
/// The rows the `inherits` relation must give for its fixture.
mod inherits {
    include!("../../tests/fixtures/relations/inherits/expected.rs");
}
/// The rows the `reference` relation must give for its fixture.
mod reference {
    include!("../../tests/fixtures/relations/reference/expected.rs");
}
/// The rows the `scope` relation must give for its fixture.
mod scope {
    include!("../../tests/fixtures/relations/scope/expected.rs");
}
/// The rows the `comment` relation must give for its fixture.
mod comment {
    include!("../../tests/fixtures/relations/comment/expected.rs");
}
/// The rows the `directive` relation must give for its fixture.
mod directive {
    include!("../../tests/fixtures/relations/directive/expected.rs");
}
/// The rows the `function` relation must give for its fixture.
mod function {
    include!("../../tests/fixtures/relations/function/expected.rs");
}
/// The rows the `parameter` relation must give for its fixture.
mod parameter {
    include!("../../tests/fixtures/relations/parameter/expected.rs");
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
/// The rows the `visibility` relation must give for its fixture.
mod visibility {
    include!("../../tests/fixtures/relations/visibility/expected.rs");
}
/// The expected rows of every relation the language reads from trees, one entry for each.
pub(super) fn expected() -> Vec<Expected> {
    vec![
        assigns::expected(),
        defines::expected(),
        inherits::expected(),
        reference::expected(),
        scope::expected(),
        comment::expected(),
        directive::expected(),
        function::expected(),
        parameter::expected(),
        static_call::expected(),
        call_argument::expected(),
        static_import::expected(),
        string_literal::expected(),
        syntax_child::expected(),
        syntax_error::expected(),
        syntax_tree::expected(),
        visibility::expected(),
    ]
}

/// Why a JavaScript declaration is never of a kind: ECMA-262 has no such declaration.
const NO_SUCH_DECLARATION: Why = Why(
    "ECMA-262 declares functions, classes, methods, fields, variables, constants, imports and labels only: no struct, enum, trait, interface, type, namespace, package, macro, section or implementation",
);

/// Why no `defines` row is a parameter in JavaScript.
const PARAMETERS_ELSEWHERE: Why = Why(
    "a JavaScript function's parameters are the rows of `parameter`, which owns them by the function",
);

/// Why a JavaScript parameter is never of a kind.
const NO_SUCH_PARAMETER: Why = Why(
    "ECMA-262 passes arguments by position only: no receiver is declared (`this` is implicit), no argument is passed by keyword, and no result is named",
);

/// The columns of the relations the language reads that its fixtures cannot show filled, each with why it never applies.
pub(super) fn unfilled() -> Vec<Unfilled> {
    let mut unfilled = vec![
        Unfilled {
            relation: "directive",
            column: "text",
            unshown: Unshown::Filled,
            reason: Why(
                "an ECMA-262 directive is a string literal alone (`\"use strict\";`): it takes no argument",
            ),
        },
        Unfilled {
            relation: "defines",
            column: "kind",
            unshown: Unshown::Word("parameter"),
            reason: PARAMETERS_ELSEWHERE,
        },
    ];
    for word in [
        "struct",
        "enum",
        "enum-member",
        "trait",
        "interface",
        "type",
        "type-alias",
        "type-parameter",
        "namespace",
        "package",
        "macro",
        "section",
        "implementation",
    ] {
        unfilled.push(Unfilled {
            relation: "defines",
            column: "kind",
            unshown: Unshown::Word(word),
            reason: NO_SUCH_DECLARATION,
        });
    }
    for word in [
        "receiver",
        "positional-or-keyword",
        "keyword-only",
        "variadic-keyword",
        "result",
    ] {
        unfilled.push(Unfilled {
            relation: "parameter",
            column: "kind",
            unshown: Unshown::Word(word),
            reason: NO_SUCH_PARAMETER,
        });
    }
    unfilled.extend([
        Unfilled {
            relation: "package_dependency",
            column: "condition",
            unshown: Unshown::Filled,
            reason: Why(
                "a package.json dependency applies everywhere: npm, pnpm and bun write no condition on one (`os` and `cpu` are the package's own)",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "interpreter",
            unshown: Unshown::Filled,
            reason: Why(
                "a package.json dependency names no version of Node: `engines` is the package's own, never one dependency's",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "kind",
            unshown: Unshown::Word("build"),
            reason: Why(
                "package.json has `dependencies`, `devDependencies`, `peerDependencies` and `optionalDependencies`, and no build dependency",
            ),
        },
    ]);
    for column in ["branch", "tag"] {
        unfilled.push(Unfilled {
            relation: "package_dependency",
            column,
            unshown: Unshown::Filled,
            reason: Why(
                "a package.json git spec names a commit-ish after its `#` (`github:owner/repository#v1.2.0`), which npm reads as a branch, a tag or a commit without the spec saying which: it is the `revision`",
            ),
        });
    }
    unfilled
}
