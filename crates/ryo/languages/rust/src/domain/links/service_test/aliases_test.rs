//! A `use` that renames binds a path only where the innermost block or inline module that holds the `use` holds the path, with no
//! inline module between them; elsewhere the path names what it writes.
use super::{Failure, Input, Repository, Value, declares, facts, subject, writes};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::module_tree::types::{Segment, Spelling, Written};
use ryo::languages::{Offset, Span};

/// `engine::query::f`, written at `place` inside the inline modules `enclosing`.
fn engine_path(enclosing: &[&str], place: (u32, u32)) -> Written {
    Written {
        enclosing: enclosing
            .iter()
            .map(|each| Segment((*each).into()))
            .collect(),
        start: Offset(place.0),
        end: Offset(place.1),
        ..writes(&["engine", "query", "f"])
    }
}

/// `use crate::shared as engine;`, its leaf written at `place` inside the inline modules `enclosing` and, when it is written in one,
/// the block of statements `block`.
fn engine_alias(enclosing: &[&str], place: (u32, u32), block: Option<(u32, u32)>) -> Written {
    Written {
        segments: vec![Segment("crate".into()), Segment("shared".into())],
        text: Spelling("crate::sharedasengine".into()),
        bound: Some(Segment("engine".into())),
        block: block.map(|(start, end)| Span {
            start: Offset(start),
            end: Offset(end),
        }),
        ..engine_path(enclosing, place)
    }
}

/// Two crates: `a` declares `shared` and depends on `b`, which it writes `engine` and which declares `query`. `a/src/lib.rs` writes
/// the `use` and the path `written`.
fn aliased(written: Vec<Written>) -> Repository {
    Repository {
        files: vec![
            "a/Cargo.toml",
            "a/src/lib.rs",
            "a/src/shared.rs",
            "b/Cargo.toml",
            "b/src/lib.rs",
            "b/src/query.rs",
        ],
        roots: vec![("a", "a/src/lib.rs"), ("b", "b/src/lib.rs")],
        outputs: Vec::new(),
        facts: vec![
            ("a/src/lib.rs", facts(declares(&["shared"]), written)),
            ("a/src/shared.rs", facts(Vec::new(), Vec::new())),
            ("b/src/lib.rs", facts(declares(&["query"]), Vec::new())),
            ("b/src/query.rs", facts(Vec::new(), Vec::new())),
        ],
        externs: vec![("a", "engine", "b", "b/src/lib.rs")],
    }
}

/// `fn a() { use crate::shared as engine; } fn b() { engine::query::f(); }`
fn an_alias_in_one_function_and_the_path_in_a_sibling() -> Repository {
    aliased(vec![
        engine_alias(&[], (13, 36), Some((7, 39))),
        engine_path(&[], (49, 65)),
    ])
}

/// `use crate::shared as engine; mod inner { fn b() { engine::query::f(); } }`
fn an_alias_at_the_file_top_and_the_path_in_an_inline_module() -> Repository {
    aliased(vec![
        engine_alias(&[], (4, 27), None),
        engine_path(&["inner"], (50, 66)),
    ])
}

/// `fn a() { use crate::shared as engine; engine::query::f(); }`
fn an_alias_and_the_path_in_the_same_block() -> Repository {
    aliased(vec![
        engine_alias(&[], (13, 36), Some((7, 59))),
        engine_path(&[], (38, 54)),
    ])
}

/// `#[cfg(test)] mod tests { use crate::shared as engine; } fn b() { engine::query::f(); }`
fn an_alias_in_a_test_module_and_the_path_outside_it() -> Repository {
    aliased(vec![
        engine_alias(&["tests"], (29, 52), None),
        engine_path(&[], (65, 81)),
    ])
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_alias_in_one_function_leaves_a_path_in_a_sibling_function_to_the_crate"),
        word: Word::Succeeds,
        input: || an_alias_in_one_function_and_the_path_in_a_sibling,
        expect: Expect::Value(
            "a/src/lib.rs -> a/src/shared.rs, a/src/lib.rs -> b/src/query.rs | a/src/lib.rs a/src/shared.rs b/src/lib.rs b/src/query.rs",
        ),
    },
    Case {
        name: CaseName("an_alias_at_the_file_top_leaves_a_path_in_an_inline_module_to_the_crate"),
        word: Word::Succeeds,
        input: || an_alias_at_the_file_top_and_the_path_in_an_inline_module,
        expect: Expect::Value(
            "a/src/lib.rs -> a/src/shared.rs, a/src/lib.rs -> b/src/query.rs | a/src/lib.rs a/src/shared.rs b/src/lib.rs b/src/query.rs",
        ),
    },
    Case {
        name: CaseName("an_alias_in_a_block_binds_a_path_in_the_same_block"),
        word: Word::Succeeds,
        input: || an_alias_and_the_path_in_the_same_block,
        expect: Expect::Value(
            "a/src/lib.rs -> a/src/shared.rs, a/src/lib.rs -> a/src/shared.rs | a/src/lib.rs a/src/shared.rs b/src/lib.rs b/src/query.rs",
        ),
    },
    Case {
        name: CaseName("an_alias_in_a_test_module_leaves_a_path_outside_it_to_the_crate"),
        word: Word::Succeeds,
        input: || an_alias_in_a_test_module_and_the_path_outside_it,
        expect: Expect::Value(
            "a/src/lib.rs -> a/src/shared.rs, a/src/lib.rs -> b/src/query.rs | a/src/lib.rs a/src/shared.rs b/src/lib.rs b/src/query.rs",
        ),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
