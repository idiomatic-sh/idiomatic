//! The expression-type index is lazy, memoized and cleared on source replacement.
use super::{
    service::prepare,
    types::{BasicType, ExpressionTypeState, Resolution},
};
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Tree};

fn is_lazy_retained_and_invalidated_between_files() {
    let first = parsed(b"x = 1 + 2.0\n").expect("source parses");
    let first_tree = Tree {
        elements: &first.elements,
        source: ByteSlice(first.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&first_tree, &mut model);
    assert_eq!(
        model.expression_types.borrow().state,
        ExpressionTypeState::Fresh
    );
    let index = prepare(&first_tree, &model);
    assert_eq!(index.state, ExpressionTypeState::Complete);
    assert!(
        index
            .entries
            .iter()
            .flatten()
            .any(|value| *value == Resolution::Atom(BasicType::Float))
    );
    let capacity = index.entries.capacity();
    drop(index);

    let second = parsed(b"x = 'changed'\n").expect("replacement source parses");
    let second_tree = Tree {
        elements: &second.elements,
        source: ByteSlice(second.bytes),
        file: languages::FileId(0),
    };
    initialize_model(&second_tree, &mut model);
    assert_eq!(
        model.expression_types.borrow().state,
        ExpressionTypeState::Fresh
    );
    let replacement = prepare(&second_tree, &model);
    assert!(replacement.entries.capacity() >= capacity);
    assert!(
        replacement
            .entries
            .iter()
            .flatten()
            .any(|value| *value == Resolution::Atom(BasicType::String))
    );
}

fn basic_type_labels_remain_stable() {
    for (kind, expected) in [
        (BasicType::Boolean, "boolean"),
        (BasicType::Integer, "integer"),
        (BasicType::Float, "float"),
        (BasicType::Complex, "complex"),
        (BasicType::String, "string"),
        (BasicType::Bytes, "bytes"),
        (BasicType::List, "list"),
        (BasicType::Tuple, "tuple"),
        (BasicType::Set, "set"),
        (BasicType::Dict, "dict"),
        (BasicType::Generator, "generator"),
        (BasicType::Null, "null"),
        (BasicType::Ellipsis, "ellipsis"),
        (BasicType::Unrecognized, "unknown"),
    ] {
        assert_eq!(super::constants::name(kind).0, expected);
    }
}
fn literal_boundary_distinguishes_unknown_from_known_types() {
    for (text, expected) in [
        ("boolean", BasicType::Boolean),
        ("integer", BasicType::Integer),
        ("float", BasicType::Float),
        ("complex", BasicType::Complex),
        ("string", BasicType::String),
        ("bytes", BasicType::Bytes),
        ("null", BasicType::Null),
        ("ellipsis", BasicType::Ellipsis),
        ("list", BasicType::Unrecognized),
        ("missing", BasicType::Unrecognized),
    ] {
        assert_eq!(
            super::constants::literal_type(languages::NameText(text)),
            expected
        );
    }
    assert_ne!(
        Resolution::Atom(BasicType::Unrecognized),
        Resolution::Unknown
    );
}
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("is_lazy_retained_and_invalidated_between_files"),
        word: Word::Succeeds,
        input: || is_lazy_retained_and_invalidated_between_files,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("basic_type_labels_remain_stable"),
        word: Word::Succeeds,
        input: || basic_type_labels_remain_stable,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("literal_boundary_distinguishes_unknown_from_known_types"),
        word: Word::Succeeds,
        input: || literal_boundary_distinguishes_unknown_from_known_types,
        expect: Expect::Value(()),
    },
];
fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
