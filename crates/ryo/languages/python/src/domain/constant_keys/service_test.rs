//! Independent boundary checks for typed constant keys.

use super::service::{clear, intern_with_hash, key_of, prepare};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::constant_keys::types::{KeyClass, RetainedKeyIndex};
use crate::domain::{
    constant_keys::types::{KeyHash, KeyState, KeyWord, LiteralKind},
    model::types::Declarations,
    semantic_syntax::{
        service::{positions, visibility},
        types::Visibility,
    },
    syntax::service::{kind, written},
    syntax::types::Kind,
};
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Tree};

fn supported_numeric_keys_normalize_and_unsupported_expressions_are_absent() {
    let file = parsed(b"1\nTrue\n1.0\n0x1\n1+0j\n1+1\n(f(), 2)\n(1, (2,))\n(1.0, (2,))\n(18446744073709551616, 1)\n(18446744073709551616, 1.0)\n(-18446744073709551616, 1)\n(-18446744073709551616, 1.0)\n18446744073709551615\n0xffffffffffffffff\n1_000\n0x3e8\n-True\n-1\n")
        .expect("independent source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(&tree, &model.constant_keys, &mut symbols, |at, symbols| {
        (visibility(&tree, &model, at, symbols) == Visibility::Visible).into()
    });
    assert_eq!(
        index.state,
        KeyState::Complete,
        "known unsupported expressions are not partial"
    );
    let occurrences: Vec<_> = positions(&tree)
        .filter(|at| {
            matches!(
                kind(&tree, *at),
                Kind::Node(
                    crate::domain::nodes::types::Node::Constant
                        | crate::domain::nodes::types::Node::Tuple
                        | crate::domain::nodes::types::Node::UnaryOp
                        | crate::domain::nodes::types::Node::BinOp
                )
            )
        })
        .map(|at| (written(&tree, at).0, key_of(&index, at)))
        .collect();
    let find = |source: &str| {
        occurrences
            .iter()
            .find(|(text, _)| *text == source)
            .and_then(|(_, key)| *key)
    };
    let equivalent = |left: &str, right: &str| {
        let left = find(left).expect("left expression has a constant key");
        let right = find(right).expect("right expression has a constant key");
        left == right
    };
    assert!(equivalent("1", "True"));
    assert!(equivalent("1", "1.0"));
    assert!(equivalent("1", "0x1"));
    assert!(equivalent("1", "1+0j"));
    assert!(equivalent("18446744073709551615", "0xffffffffffffffff"));
    assert!(equivalent("1_000", "0x3e8"));
    assert!(equivalent("-True", "-1"));
    assert!(equivalent(
        "(18446744073709551616, 1)",
        "(18446744073709551616, 1.0)"
    ));
    assert!(find("1+1").is_none());
    assert!(find("(f(), 2)").is_none());
    assert!(find("(-18446744073709551616, 1)").is_none());
    assert!(find("(-18446744073709551616, 1.0)").is_none());
    assert!(equivalent("(1, (2,))", "(1.0, (2,))"));
}

fn float_boundaries_and_huge_integer_structural_keys_are_retained() {
    let file = parsed(b"9007199254740992\n9007199254740992.0\n9007199254740993\n9007199254740993.0\n18446744073709551616\n18446744073709551616\n-0.0\n0.0\n")
        .expect("independent source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(&tree, &model.constant_keys, &mut symbols, |at, symbols| {
        (visibility(&tree, &model, at, symbols) == Visibility::Visible).into()
    });
    let values: Vec<_> = positions(&tree)
        .filter(|at| {
            matches!(
                kind(&tree, *at),
                Kind::Node(
                    crate::domain::nodes::types::Node::Constant
                        | crate::domain::nodes::types::Node::UnaryOp
                )
            )
        })
        .map(|at| (written(&tree, at).0, key_of(&index, at)))
        .collect();
    let keys: Vec<_> = [
        "9007199254740992",
        "9007199254740992.0",
        "9007199254740993",
        "9007199254740993.0",
        "18446744073709551616",
        "18446744073709551616",
        "-0.0",
        "0.0",
    ]
    .iter()
    .map(|wanted| {
        values
            .iter()
            .find(|(text, _)| text == wanted)
            .and_then(|(_, key)| *key)
    })
    .collect();
    for key in &keys {
        let _key = key.expect("each supported boundary expression has a key");
    }
    assert_eq!(keys[0], keys[1]);
    assert_ne!(keys[2], keys[3]);
    let huge: Vec<_> = values
        .iter()
        .filter(|(text, _)| *text == "18446744073709551616")
        .filter_map(|(_, key)| *key)
        .collect();
    assert_eq!(huge.len(), 2);
    assert_eq!(huge[0], huge[1]);
    assert_eq!(keys[6], keys[7]);
}

fn underflow_subnormal_infinity_complex_and_unary_values_follow_typed_payloads() {
    let source = b"0\n1e-9999\n5e-324\n1e9999\n0j\n1j\n1+1j\n1.0+1j\n1-1j\n1.0-1j\n9007199254740993+0j\n9007199254740992\n-0.0\n0.0\n18446744073709551615\n18446744073709551616.0\n";
    let file = parsed(source).expect("independent source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(&tree, &model.constant_keys, &mut symbols, |at, symbols| {
        (visibility(&tree, &model, at, symbols) == Visibility::Visible).into()
    });
    let occurrences: Vec<_> = positions(&tree)
        .filter(|at| {
            matches!(
                kind(&tree, *at),
                Kind::Node(
                    crate::domain::nodes::types::Node::Constant
                        | crate::domain::nodes::types::Node::UnaryOp
                        | crate::domain::nodes::types::Node::BinOp
                )
            )
        })
        .map(|at| (written(&tree, at).0, key_of(&index, at)))
        .collect();
    let find = |spelling: &str| {
        occurrences
            .iter()
            .find(|(text, _)| *text == spelling)
            .and_then(|(_, key)| *key)
    };
    let key = |spelling: &str| find(spelling).expect("supported value has a constant key");
    assert_eq!(key("0"), key("1e-9999"));
    assert_ne!(key("0"), key("5e-324"));
    assert_ne!(key("0"), key("1e9999"));
    assert_eq!(key("0"), key("0j"));
    assert_ne!(key("0"), key("1j"));
    assert_eq!(key("1+1j"), key("1.0+1j"));
    assert_eq!(key("1-1j"), key("1.0-1j"));
    assert_eq!(key("9007199254740993+0j"), key("9007199254740992"));
    assert_eq!(key("-0.0"), key("0.0"));
    assert_ne!(key("18446744073709551615"), key("18446744073709551616.0"));
}

fn invalidation_clears_occurrences_but_keeps_the_reader_reusable() {
    let mut model = Declarations::default();
    let first = parsed(b"1\n").expect("source parses");
    let first_tree = Tree {
        elements: &first.elements,
        source: ByteSlice(first.bytes),
        file: languages::FileId(0),
    };
    let mut first_symbols = first.symbols;
    initialize_model(&first_tree, &mut model);
    assert!(
        model.constant_keys.borrow().records.is_empty(),
        "parsing is lazy"
    );
    let first_index = prepare(
        &first_tree,
        &model.constant_keys,
        &mut first_symbols,
        |at, symbols| (visibility(&first_tree, &model, at, symbols) == Visibility::Visible).into(),
    );
    let first_words = first_index.words.clone();
    assert!(!first_index.records.is_empty());
    drop(first_index);

    clear(model.constant_keys.get_mut());
    let second = parsed(b"2\n").expect("replacement source parses");
    let second_tree = Tree {
        elements: &second.elements,
        source: ByteSlice(second.bytes),
        file: languages::FileId(0),
    };
    let mut second_symbols = second.symbols;
    initialize_model(&second_tree, &mut model);
    let second_index = prepare(
        &second_tree,
        &model.constant_keys,
        &mut second_symbols,
        |at, symbols| (visibility(&second_tree, &model, at, symbols) == Visibility::Visible).into(),
    );
    assert!(!second_index.records.is_empty());
    assert_ne!(second_index.words, first_words);
}

fn full_keys_disambiguate_hash_collisions() {
    let mut index = RetainedKeyIndex::<KeyClass, KeyWord, KeyHash, KeyState>::default();
    index.scratch.push(KeyWord::Literal(
        LiteralKind::Integer,
        Some(languages::Symbol(10)),
    ));
    let first = intern_with_hash(&mut index, languages::Position(2), KeyHash(0))
        .expect("first key interns");
    index.scratch.clear();
    index.scratch.push(KeyWord::Literal(
        LiteralKind::Integer,
        Some(languages::Symbol(11)),
    ));
    // Test the collision chain directly by using the same bucket hash in the service's intern path.
    let second = intern_with_hash(&mut index, languages::Position(5), KeyHash(0))
        .expect("second key interns");
    assert_ne!(first, second);
    assert_eq!(index.records.len(), 2);
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("supported_numeric_keys_normalize_and_unsupported_expressions_are_absent"),
        word: Word::Succeeds,
        input: || supported_numeric_keys_normalize_and_unsupported_expressions_are_absent,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("float_boundaries_and_huge_integer_structural_keys_are_retained"),
        word: Word::Succeeds,
        input: || float_boundaries_and_huge_integer_structural_keys_are_retained,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "underflow_subnormal_infinity_complex_and_unary_values_follow_typed_payloads",
        ),
        word: Word::Succeeds,
        input: || underflow_subnormal_infinity_complex_and_unary_values_follow_typed_payloads,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalidation_clears_occurrences_but_keeps_the_reader_reusable"),
        word: Word::Succeeds,
        input: || invalidation_clears_occurrences_but_keeps_the_reader_reusable,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("full_keys_disambiguate_hash_collisions"),
        word: Word::Succeeds,
        input: || full_keys_disambiguate_hash_collisions,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
