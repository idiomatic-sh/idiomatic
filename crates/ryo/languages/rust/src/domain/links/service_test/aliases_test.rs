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

/// `use crate::<module> as engine;`, its leaf written at `place` inside the inline modules `enclosing`, binding in `scope`: the
/// innermost block or inline module around it, or the whole file when none.
fn engine_alias(
    module: &str,
    enclosing: &[&str],
    place: (u32, u32),
    scope: Option<(u32, u32)>,
) -> Written {
    Written {
        segments: vec![Segment("crate".into()), Segment(module.into())],
        text: Spelling(format!("crate::{module}asengine").into()),
        bound: Some(Segment("engine".into())),
        scope: scope.map(|(start, end)| Span {
            start: Offset(start),
            end: Offset(end),
        }),
        ..engine_path(enclosing, place)
    }
}

/// Two crates: `a` declares `other` and `shared` and depends on `b`, which it writes `engine` and which declares `query`.
/// `a/src/lib.rs` writes the `use`s and the path `written`.
fn aliased(written: Vec<Written>) -> Repository {
    Repository {
        files: vec![
            "a/Cargo.toml",
            "a/src/lib.rs",
            "a/src/other.rs",
            "a/src/shared.rs",
            "b/Cargo.toml",
            "b/src/lib.rs",
            "b/src/query.rs",
        ],
        roots: vec![("a", "a/src/lib.rs"), ("b", "b/src/lib.rs")],
        outputs: Vec::new(),
        facts: vec![
            (
                "a/src/lib.rs",
                facts(declares(&["other", "shared"]), written),
            ),
            ("a/src/other.rs", facts(Vec::new(), Vec::new())),
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
        engine_alias("shared", &[], (13, 36), Some((7, 39))),
        engine_path(&[], (49, 65)),
    ])
}

/// `use crate::shared as engine; mod inner { fn b() { engine::query::f(); } }`
fn an_alias_at_the_file_top_and_the_path_in_an_inline_module() -> Repository {
    aliased(vec![
        engine_alias("shared", &[], (4, 27), None),
        engine_path(&["inner"], (50, 66)),
    ])
}

/// `fn a() { use crate::shared as engine; engine::query::f(); }`
fn an_alias_and_the_path_in_the_same_block() -> Repository {
    aliased(vec![
        engine_alias("shared", &[], (13, 36), Some((7, 59))),
        engine_path(&[], (38, 54)),
    ])
}

/// `#[cfg(test)] mod tests { use crate::shared as engine; } fn b() { engine::query::f(); }`
fn an_alias_in_a_test_module_and_the_path_outside_it() -> Repository {
    aliased(vec![
        engine_alias("shared", &["tests"], (29, 52), Some((23, 55))),
        engine_path(&[], (65, 81)),
    ])
}

/// `use crate::shared as engine; fn a() { use crate::other as engine; engine::query::f(); }`
fn an_alias_in_a_block_under_one_at_the_file_top() -> Repository {
    aliased(vec![
        engine_alias("shared", &[], (4, 27), None),
        engine_alias("other", &[], (42, 64), Some((36, 87))),
        engine_path(&[], (66, 82)),
    ])
}

/// `fn a() { mod m { use crate::shared as engine; } } fn b() { mod m { fn c() { engine::query::f(); } } }`
fn an_alias_in_an_inline_module_and_the_path_in_another_of_the_same_name() -> Repository {
    aliased(vec![
        engine_alias("shared", &["m"], (21, 44), Some((15, 47))),
        engine_path(&["m"], (76, 92)),
    ])
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_alias_in_one_function_leaves_a_path_in_a_sibling_function_to_the_crate"),
        word: Word::Succeeds,
        input: || an_alias_in_one_function_and_the_path_in_a_sibling,
        expect: Expect::Value(TO_THE_CRATE),
    },
    Case {
        name: CaseName("an_alias_at_the_file_top_leaves_a_path_in_an_inline_module_to_the_crate"),
        word: Word::Succeeds,
        input: || an_alias_at_the_file_top_and_the_path_in_an_inline_module,
        expect: Expect::Value(TO_THE_CRATE),
    },
    Case {
        name: CaseName("an_alias_in_a_block_binds_a_path_in_the_same_block"),
        word: Word::Succeeds,
        input: || an_alias_and_the_path_in_the_same_block,
        expect: Expect::Value(
            "a/src/lib.rs -> a/src/shared.rs, a/src/lib.rs -> a/src/shared.rs | a/src/lib.rs a/src/other.rs a/src/shared.rs b/src/lib.rs b/src/query.rs",
        ),
    },
    Case {
        name: CaseName("an_alias_in_a_test_module_leaves_a_path_outside_it_to_the_crate"),
        word: Word::Succeeds,
        input: || an_alias_in_a_test_module_and_the_path_outside_it,
        expect: Expect::Value(TO_THE_CRATE),
    },
    Case {
        name: CaseName("an_alias_in_a_block_shadows_one_of_the_same_name_at_the_file_top"),
        word: Word::Succeeds,
        input: || an_alias_in_a_block_under_one_at_the_file_top,
        expect: Expect::Value(
            "a/src/lib.rs -> a/src/other.rs, a/src/lib.rs -> a/src/other.rs, a/src/lib.rs -> a/src/shared.rs | a/src/lib.rs a/src/other.rs a/src/shared.rs b/src/lib.rs b/src/query.rs",
        ),
    },
    Case {
        name: CaseName(
            "an_alias_in_an_inline_module_leaves_a_path_in_another_module_of_the_same_name_to_the_crate",
        ),
        word: Word::Succeeds,
        input: || an_alias_in_an_inline_module_and_the_path_in_another_of_the_same_name,
        expect: Expect::Value(TO_THE_CRATE),
    },
];

/// The `use` names its module and the path names the crate `b`'s `query`.
const TO_THE_CRATE: &str = "a/src/lib.rs -> a/src/shared.rs, a/src/lib.rs -> b/src/query.rs | a/src/lib.rs a/src/other.rs a/src/shared.rs b/src/lib.rs b/src/query.rs";

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
