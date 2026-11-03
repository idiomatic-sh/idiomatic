//! Literal-mask setup work, exact vocabulary identity and generation isolation.

use crate::domain::kind_sets::service::{kind_tests, passes, prepare_vocabulary};
use crate::domain::kind_sets::types::KindScratch;
use crate::domain::operating::types::OperatorContext;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::Verdict;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::relations::types::{FileId, KindBits, Offset};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, LiteralText, Operand, QueryLiteral, Test,
};

/// One authored kind comparison at the supplied input column.
fn compared(test: Test, place: u32, literal: &str) -> Comparison {
    Comparison {
        test,
        left: Operand::Column(column(place)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(literal.into()))),
        origin: None,
    }
}

/// A typed column index used by the fixture.
fn column(place: u32) -> ColumnIndex {
    ColumnIndex(place)
}

/// Build a small real vocabulary; the payload is irrelevant to prepared kind predicates.
fn prepared_tree(names: &[&str]) -> (Elements, Symbols) {
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"a"));
    enter_node(&mut builder, KindBits(0), Offset(0));
    token(&mut builder, KindBits(1), Offset(0), Offset(1));
    leave_node(&mut builder, Offset(1));
    for (index, name) in names.iter().enumerate() {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(b"a"), FileId(0)),
            KindBits(u16::try_from(index).unwrap()),
            NameText(name),
        );
    }
    (elements, symbols)
}

/// A literal mask is computed once across files, with no metadata lock in row admission.
#[test]
fn prepared_masks_reuse_exact_vocabulary_without_per_row_setup() {
    use std::sync::atomic::Ordering;
    let comparisons = vec![compared(Test::GlobAny, 1, "wor*")];
    let mut previous = None;
    let mut last_set = None;
    for file in 0..2 {
        let (elements, mut symbols) = prepared_tree(&["file", "word"]);
        let prepared =
            prepare_vocabulary(&mut previous, &elements, &symbols, SourceView(b"a")).unwrap();
        let tree = TreeRows {
            elements: &elements,
            file: FileId(file),
            form: TreeForm::Elements,
        };
        let mut kinds = KindScratch {
            names: Default::default(),
            sets: Vec::new(),
            tests: Default::default(),
        };
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            prepared_kinds: Some(&prepared),
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(b"a"),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        kind_tests(&tree, &comparisons, &mut context, &mut kinds);
        let lookups = prepared.lookups.load(Ordering::Relaxed);
        for _row in 0..1000 {
            assert_eq!(passes(&kinds, KindBits(1)), Verdict::Holds);
            assert_eq!(passes(&kinds, KindBits(0)), Verdict::Fails);
        }
        kind_tests(&tree, &comparisons, &mut context, &mut kinds);
        assert_eq!(prepared.lookups.load(Ordering::Relaxed), lookups);
        assert_eq!(prepared.builds.load(Ordering::Relaxed), 1);
        assert_eq!(prepared.masks.lock().unwrap().len(), 1);
        let set = kinds.sets.first().unwrap();
        if let Some(previous) = last_set.as_ref() {
            assert!(std::sync::Arc::ptr_eq(previous, set));
        }
        last_set = Some((set).clone());
    }
}

/// Exact name numbering replaces the generation; old active handles retain their own immutable masks.
#[test]
fn prepared_vocabulary_replacement_preserves_live_old_masks_and_test_identity() {
    let comparisons = vec![
        compared(Test::Equal, 1, "word"),
        compared(Test::NotEqual, 1, "word"),
    ];
    let mut previous = None;
    let mut first = None;
    for names in [["file", "word"], ["word", "file"], ["file", "word"]] {
        let (elements, mut symbols) = prepared_tree(&names);
        let prepared =
            prepare_vocabulary(&mut previous, &elements, &symbols, SourceView(b"a")).unwrap();
        if let Some(first) = first.as_ref() {
            assert!(!std::sync::Arc::ptr_eq(first, &prepared));
        }
        let mut kinds = KindScratch {
            names: Default::default(),
            sets: Vec::new(),
            tests: Default::default(),
        };
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            prepared_kinds: Some(&prepared),
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(b"a"),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        kind_tests(
            &TreeRows {
                elements: &elements,
                file: FileId(0),
                form: TreeForm::Elements,
            },
            &comparisons,
            &mut context,
            &mut kinds,
        );
        assert_eq!(prepared.masks.lock().unwrap().len(), 2);
        // Equal and not-equal cannot alias merely because their literal text is the same.
        assert_eq!(passes(&kinds, KindBits(0)), Verdict::Fails);
        assert_eq!(passes(&kinds, KindBits(1)), Verdict::Fails);
        if first.is_none() {
            first = Some((prepared).clone());
        }
    }
    let first = first.unwrap();
    let masks = first.masks.lock().unwrap();
    assert_eq!(masks.len(), 2);
    assert!(masks.values().any(|set| set.kinds == [KindBits(1)]));
    assert!(masks.values().any(|set| set.kinds == [KindBits(0)]));
}

/// A real collision in the old additive KindSum cannot reuse a differently numbered vocabulary.
#[test]
fn equal_kind_sums_do_not_identify_prepared_vocabularies() {
    use ryo_language::domain::elements::service::kind_vocabulary;
    use ryo_language::domain::symbols::service::hash_of;
    let names = ["word", "file", "node"];
    let (left, right) = names
        .iter()
        .enumerate()
        .find_map(|(index, first)| {
            names
                .iter()
                .skip(index + 1)
                .find(|second| {
                    hash_of(ByteSlice(first.as_bytes())).0 & 1
                        == hash_of(ByteSlice(second.as_bytes())).0 & 1
                })
                .map(|second| (*first, *second))
        })
        .unwrap();
    let (first, first_symbols) = prepared_tree(&[left, right]);
    let (second, second_symbols) = prepared_tree(&[right, left]);
    assert_eq!(kind_vocabulary(&first), kind_vocabulary(&second));
    let mut previous = None;
    let a = prepare_vocabulary(&mut previous, &first, &first_symbols, SourceView(b"a")).unwrap();
    let b = prepare_vocabulary(&mut previous, &second, &second_symbols, SourceView(b"a")).unwrap();
    assert!(!std::sync::Arc::ptr_eq(&a, &b));
    assert_ne!(a.names, b.names);
    assert_eq!(a.names.first().unwrap().as_deref(), Some(left));
    assert_eq!(b.names.first().unwrap().as_deref(), Some(right));
}

/// Runtime operands are not admitted to finite prepared literal metadata.
#[test]
fn dynamic_kind_patterns_do_not_enter_prepared_metadata() {
    let (elements, mut symbols) = prepared_tree(&["file", "word"]);
    let prepared = prepare_vocabulary(&mut None, &elements, &symbols, SourceView(b"a")).unwrap();
    let comparisons = vec![Comparison {
        test: Test::Matches,
        left: Operand::Column(column(1)),
        right: Operand::Column(column(2)),
        origin: None,
    }];
    let mut kinds = KindScratch {
        names: Default::default(),
        sets: Vec::new(),
        tests: Default::default(),
    };
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        prepared_kinds: Some(&prepared),
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b"a"),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    kind_tests(
        &TreeRows {
            elements: &elements,
            file: FileId(0),
            form: TreeForm::Elements,
        },
        &comparisons,
        &mut context,
        &mut kinds,
    );
    assert!(prepared.masks.lock().unwrap().is_empty());
    assert!(kinds.tests.0.is_empty());
    assert_eq!(
        prepared.lookups.load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

/// Prepared metadata can cross worker ownership boundaries without retaining local source borrows.
#[test]
fn prepared_vocabulary_is_send_and_sync() {
    fn requires_send_sync<T: Send + Sync>() {}
    requires_send_sync::<crate::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>();
}

/// Prepared and uncached operators return the same ordered cells, repeated projections, and schemas.
#[test]
fn prepared_masks_match_uncached_select_outputs() {
    use crate::application::file_dag::testing::TestOperator as _;
    use crate::domain::operator::collections::operators::select::item::SelectItem;
    use crate::domain::table_model::types::{Rows, Table};
    use ryo_language::domain::relations::types::Relation;
    use ryo_values::domain::plan_parts::types::Input;
    use ryo_values::domain::plans::types::Operation;
    let (elements, mut symbols) = prepared_tree(&["file", "word"]);
    let prepared = prepare_vocabulary(&mut None, &elements, &symbols, SourceView(b"a")).unwrap();
    for (test, text) in [
        (Test::Equal, "word"),
        (Test::NotEqual, "file"),
        (Test::GlobAny, "wor*"),
        (Test::NotGlobAny, "fi*"),
        (Test::StartsWith, "wo"),
        (Test::EndsWith, "rd"),
        (Test::Matches, "^word$"),
        (Test::Equal, "absent"),
    ] {
        let operation = Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            comparisons: vec![compared(test, 1, text)],
            kept: vec![column(0), column(1), column(0)],
        };
        let mut previous: Option<Table> = None;
        for metadata in [None, Some(prepared.as_ref())] {
            let mut scratch = OperatorScratch::default();
            let mut memo = Vec::new();
            let mut context = OperatorContext {
                prepared_kinds: metadata,
                prepared_matchers: None,
                bindings: &[],
                symbols: (&mut symbols).into(),
                source: SourceView(b"a"),
                paths: &[],
                arenas: &[],
                memo: &mut memo,
                scratch: &mut scratch,
            };
            let mut out = Table::default();
            SelectItem
                .compute(
                    &operation,
                    &[Rows::Tree(TreeRows {
                        elements: &elements,
                        file: FileId(0),
                        form: TreeForm::Elements,
                    })],
                    &mut context,
                    &mut out,
                )
                .unwrap();
            assert_eq!(out.count.0, usize::from(text != "absent"));
            if let Some(previous) = previous.as_ref() {
                assert_eq!(out.kinds, previous.kinds);
                assert_eq!(out.cells, previous.cells);
                assert_eq!(out.count, previous.count);
            }
            previous = Some(out);
        }
    }
}
