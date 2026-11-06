//! Independent AST identity examples; no reference linter supplies these expected classes.
use crate::domain::constant_keys::types::RetainedKeyIndex;
use ryo::languages;
use ryo::languages::Position;
use super::service::clear;
use super::service::intern;
use super::service::prepare;
use super::service::shape_of;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::domain::model::types::Declarations;
use crate::domain::model::types::ShapeHash;
use crate::domain::model::types::ShapeState;
use crate::domain::model::types::ShapeWord;
use crate::domain::nodes::types::Category;
use crate::domain::syntax::service::category;
use crate::domain::syntax::service::written;
use crate::fixtures::reading::initialize_model;
use crate::fixtures::reading::parsed;
use ryo::languages::ByteSlice;
use ryo::languages::Tree;
type Input = (&'static str, &'static str, &'static str);
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "relocated literal values compare logically and inactive candidates stay hidden",
        ),
        word: Word::Succeeds,
        input: || ("embedding witness", "", ""),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("equal huge integers in different radices stay structurally distinct"),
        word: Word::Succeeds,
        input: || {
            (
                "184467440737095516160\n0xA0000000000000000\n",
                "184467440737095516160",
                "0xA0000000000000000",
            )
        },
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("identical huge integer spellings have equal shapes"),
        word: Word::Succeeds,
        input: || {
            (
                "184467440737095516160\n184467440737095516160\n",
                "184467440737095516160",
                "184467440737095516160",
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unequal huge integer payloads remain distinct"),
        word: Word::Succeeds,
        input: || {
            (
                "184467440737095516160\n184467440737095516161\n",
                "184467440737095516160",
                "184467440737095516161",
            )
        },
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("float spelling normalizes through typed bits"),
        word: Word::Succeeds,
        input: || ("1e0\n1.0\n", "1e0", "1.0"),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("complex literal remains distinct from real float"),
        word: Word::Succeeds,
        input: || ("1j\n1.0\n", "1j", "1.0"),
        expect: Expect::Value(false),
    },
];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared fixture runner requires every case subject to return Result"
)]
fn subject((source, left, right): Input) -> Result<Value, Failure> {
    if source == "embedding witness" {
        return Ok(embedded_shapes());
    }
    if source == "collision witness" {
        let mut index = RetainedKeyIndex::<Position, ShapeWord, ShapeHash, ShapeState>::default();
        index
            .scratch
            .push(ShapeWord::Identifier(languages::Symbol(1)));
        let left_class = intern(&mut index, languages::Position(2), ShapeHash(0));
        index.scratch.clear();
        index
            .scratch
            .push(ShapeWord::Identifier(languages::Symbol(2)));
        let right_class = intern(&mut index, languages::Position(6), ShapeHash(0));
        index.scratch.clear();
        index
            .scratch
            .push(ShapeWord::Identifier(languages::Symbol(1)));
        let repeated_class = intern(&mut index, languages::Position(9), ShapeHash(0));
        return Ok(left_class != right_class
            && left_class == repeated_class
            && index.records.len() == 2);
    }
    if source == "reparse witness" {
        let mut model = Declarations::default();
        for (fixture_source, complete) in [
            ("x\nx\n", true),
            ("x +\ny *\n", false),
            ("", true),
            ("y\nx\n", true),
        ] {
            clear(model.expression_shapes.get_mut());
            let file = parsed(fixture_source.as_bytes()).expect("parser retains diagnostics");
            let tree = Tree {
                elements: &file.elements,
                source: ByteSlice(file.bytes),
                file: languages::FileId(0),
            };
            let mut symbols = file.symbols;
            initialize_model(&tree, &mut model);
            assert_eq!(prepare(&tree, &model, &mut symbols).is_some(), complete);
            if !complete {
                assert!(model.expression_shapes.borrow().classes.is_empty());
                assert_eq!(model.expression_shapes.borrow().state, ShapeState::Partial);
            }
        }
        return Ok(true);
    }

    let file = parsed(source.as_bytes()).expect("independently authored source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    assert!(
        model.expression_shapes.borrow().classes.is_empty(),
        "parsing alone performs no structural indexing"
    );
    let first = crate::domain::semantic_syntax::service::positions(&tree)
        .find(|at| category(&tree, *at) == Category::Expression && written(&tree, *at).0 == left)
        .expect("left occurrence");
    let last = crate::domain::semantic_syntax::service::positions(&tree)
        .filter(|at| category(&tree, *at) == Category::Expression && written(&tree, *at).0 == right)
        .last()
        .expect("right occurrence");
    let index = prepare(&tree, &model, &mut symbols).unwrap_or_else(|| {
        panic!("complete source has shapes: {source:?}; left={left:?}; right={right:?}")
    });
    let left_shape = shape_of(&index, first).expect("left shape");
    let right_shape = shape_of(&index, last).expect("right shape");
    let reentrant = prepare(&tree, &model, &mut symbols).expect("immutable read can reenter");
    assert_eq!(Some(left_shape), shape_of(&reentrant, first));
    drop(reentrant);
    drop(index);
    let repeated = prepare(&tree, &model, &mut symbols).expect("repeat read is complete");
    assert_eq!(Some(left_shape), shape_of(&repeated, first));
    drop(repeated);
    clear(&mut model.expression_shapes.borrow_mut());
    assert!(model.expression_shapes.borrow().classes.is_empty());
    let reparsed = prepare(&tree, &model, &mut symbols).expect("cleared index rebuilds");
    assert_eq!(Some(left_shape), shape_of(&reparsed, first));
    Ok(left_shape == right_shape)
}

/// Whole-host relocation must not use carrier text as the embedded literal value.
fn embedded_shapes() -> Value {
    use crate::domain::{
        semantic_syntax::{
            service::{positions, visibility},
            types::Visibility,
        },
        syntax::constants::{BOOLEAN_SCALAR, SCALAR_KIND, SCALAR_VALUE},
    };
    let file = parsed(b"True\nvalue: \"tuple[Tr\\x75e, False]\"\nordinary('hidden + value')\n")
        .expect("valid independently authored source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let expected: Vec<_> = positions(&tree)
        .map(|at| (at, visibility(&tree, &model, at, &mut symbols)))
        .collect();
    assert!(
        model.expression_shapes.borrow().classes.is_empty(),
        "semantic reads do not build structural keys"
    );
    let first = prepare(&tree, &model, &mut symbols).expect("complete semantic shapes");
    let values: Vec<_> = expected
        .iter()
        .filter_map(|(at, visible)| {
            if *visible != Visibility::Visible
                || languages::property_at(tree.elements, *at, SCALAR_KIND).map(|value| value.0)
                    != Some(BOOLEAN_SCALAR)
            {
                return None;
            }
            Some((
                languages::property_at(tree.elements, *at, SCALAR_VALUE)
                    .expect("boolean payload")
                    .0,
                shape_of(&first, *at).expect("visible boolean shape"),
            ))
        })
        .collect();
    let truths: Vec<_> = values
        .iter()
        .filter(|(value, _)| *value == "true")
        .map(|(_, shape)| *shape)
        .collect();
    let falsehoods: Vec<_> = values
        .iter()
        .filter(|(value, _)| *value == "false")
        .map(|(_, shape)| *shape)
        .collect();
    assert_eq!(truths.len(), 2);
    assert_eq!(falsehoods.len(), 1);
    assert_eq!(truths.first(), truths.last());
    assert_ne!(truths.first(), falsehoods.first());
    for (at, visible) in &expected {
        if *visible == Visibility::Hidden {
            assert!(shape_of(&first, *at).is_none());
        }
    }
    let classes = first.classes.clone();
    drop(first);
    initialize_model(&tree, &mut model);
    let shape_first = prepare(&tree, &model, &mut symbols)
        .expect("shape-first read initializes semantic visibility");
    assert_eq!(classes, shape_first.classes);
    true
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod semantic_identity_test;
#[cfg(test)]
mod strings_and_calls_test;
#[cfg(test)]
mod structure_test;
