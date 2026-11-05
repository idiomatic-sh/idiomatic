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
/// The rows the `annotates` relation must give for its fixture input.
mod annotates {
    include!("../../tests/fixtures/relations/annotates/expected.rs");
}

/// The rows the `assigns` relation must give for its fixture input.
mod assigns {
    include!("../../tests/fixtures/relations/assigns/expected.rs");
}

/// The rows the `comment` relation must give for its fixture input.
mod comment {
    include!("../../tests/fixtures/relations/comment/expected.rs");
}

/// The rows the `defines` relation must give for its fixture input.
mod defines {
    include!("../../tests/fixtures/relations/defines/expected.rs");
}

/// The rows the `directive` relation must give for its fixture input.
mod directive {
    include!("../../tests/fixtures/relations/directive/expected.rs");
}

/// The rows the `entry` relation must give for its fixture input.
mod entry {
    include!("../../tests/fixtures/relations/entry/expected.rs");
}

/// The rows the `function` relation must give for its fixture input.
mod function {
    include!("../../tests/fixtures/relations/function/expected.rs");
}

/// The rows the `inherits` relation must give for its fixture input.
mod inherits {
    include!("../../tests/fixtures/relations/inherits/expected.rs");
}

/// The rows the `mapping` relation must give for its fixture input.
mod mapping {
    include!("../../tests/fixtures/relations/mapping/expected.rs");
}

/// The rows the `parameter` relation must give for its fixture input.
mod parameter {
    include!("../../tests/fixtures/relations/parameter/expected.rs");
}

/// The rows the `reference` relation must give for its fixture input.
mod reference {
    include!("../../tests/fixtures/relations/reference/expected.rs");
}

/// The rows the `scope` relation must give for its fixture input.
mod scope {
    include!("../../tests/fixtures/relations/scope/expected.rs");
}

/// The rows the `static_call` relation must give for its fixture input.
mod static_call {
    include!("../../tests/fixtures/relations/static_call/expected.rs");
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
        annotates::expected(),
        assigns::expected(),
        comment::expected(),
        defines::expected(),
        directive::expected(),
        entry::expected(),
        function::expected(),
        inherits::expected(),
        mapping::expected(),
        parameter::expected(),
        reference::expected(),
        scope::expected(),
        static_call::expected(),
        resolves::expected(),
        call_argument::expected(),
        static_import::expected(),
        string_literal::expected(),
        syntax_child::expected(),
        semantic_child::expected(),
        syntax_error::expected(),
        syntax_tree::expected(),
        semantic_syntax::expected(),
        source_location::expected(),
    ]
}

/// The columns of the relations the language reads that its fixtures cannot show filled, each with why it never applies.
pub(super) fn unfilled() -> Vec<Unfilled> {
    let mut found = vec![
        Unfilled {
            relation: "package_dependency",
            column: "condition",
            unshown: Unshown::Filled,
            reason: Why(
                "a go.mod `require` writes no condition: the Go Modules Reference gives the directive a module path and a version only",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "optional",
            unshown: Unshown::Word("true"),
            reason: Why(
                "a go.mod `require` is never optional: minimal version selection reads every requirement of the build list",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "interpreter",
            unshown: Unshown::Filled,
            reason: Why(
                "a go.mod `require` names no Go version: the `go` and `toolchain` directives are the module's own (Go Modules Reference)",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "catalog",
            unshown: Unshown::Filled,
            reason: Why(
                "a go.mod writes every requirement itself: no directive takes a module's version from a shared table",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "exact",
            unshown: Unshown::Word("true"),
            reason: Why(
                "a go.mod `require` names a minimum version, which minimal version selection may raise (Go Modules Reference, \"Minimal version selection\"): no requirement admits one version only",
            ),
        },
    ];
    found.extend(words(
        "defines",
        "kind",
        &["class", "enum", "enum-member", "trait", "namespace", "package", "macro", "section", "implementation"],
        "the Go specification declares constants, types, variables, functions, methods, labels, imports, fields and type parameters: no class, enum, trait, namespace, macro, section or implementation, and the package clause is no declaration (\"Package clause\": the package name belongs to no scope)",
    ));
    found.extend(words(
        "parameter",
        "kind",
        &["positional-or-keyword", "keyword-only", "variadic-keyword"],
        "a Go argument is passed by position alone: the Go specification has no keyword argument (\"Calls\")",
    ));
    for column in ["branch", "tag", "revision"] {
        found.push(Unfilled {
            relation: "package_dependency",
            column,
            unshown: Unshown::Filled,
            reason: Why(
                "a go.mod names module versions only, never a branch, a tag or a revision of a repository: a commit is reached through a pseudo-version (`v0.0.0-20191109021931-daa7c04131f5`), which `requirement` holds",
            ),
        });
    }
    found.extend(words(
        "package_dependency",
        "kind",
        &["dev", "build", "peer"],
        "a go.mod `require` has no kind: the Go Modules Reference writes every requirement the same, `// indirect` included",
    ));
    found.extend(words(
        "package_dependency",
        "origin",
        &["git", "url", "workspace"],
        "a go.mod `require` names a module path, found through the module proxy, or a `replace` sends it to a directory: no requirement writes a repository, an archive's URL, or a workspace's entry",
    ));
    found.extend(module_file_unfilled());
    found
}

/// The columns of the keyed relations a `go.mod` never fills. A go.mod is a list of directives of words (Go Modules Reference, "go.mod
/// files"): a `require` keys a module path to a version, always a word; it nests no table, holds no list and writes no number, flag,
/// date or null.
fn module_file_unfilled() -> Vec<Unfilled> {
    let mut found = Vec::new();
    found.extend(words(
        "entry",
        "kind",
        &[
            "integer",
            "float",
            "boolean",
            "datetime",
            "sequence",
            "inline-mapping",
            "mapping",
            "sequence-of-mappings",
            "null",
        ],
        "a go.mod `require` keys a module path to its version, a word: the Go Modules Reference writes no other value",
    ));
    found.extend(words(
        "entry",
        "boolean",
        &["true"],
        "a go.mod writes no flag: a `require` version is a word",
    ));
    for column in ["integer", "float", "datetime", "radix"] {
        found.push(Unfilled {
            relation: "entry",
            column,
            unshown: Unshown::Filled,
            reason: Why("a go.mod writes no number and no date: a `require` version is a word (Go Modules Reference)"),
        });
    }
    found.extend(words(
        "mapping",
        "kind",
        &["dotted", "inline", "sequence-member"],
        "a go.mod's mappings are the file and its `require` blocks: no key is dotted, no table is written inline, and no list holds tables",
    ));
    found.push(Unfilled {
        relation: "mapping",
        column: "ordinal",
        unshown: Unshown::Filled,
        reason: Why("a go.mod holds no list of mappings, so no mapping has a place in one"),
    });
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
