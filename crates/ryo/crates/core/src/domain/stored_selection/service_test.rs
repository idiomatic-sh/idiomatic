//! The `stored_selection` service's cases, on values alone.

use super::service::select;
use super::types::Selector;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::coverage::types::{Lacking, Partial};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::service::compute;
use crate::domain::stored_indexes::service::{LIMIT, new};
use crate::domain::table_model::service::materialize;
use crate::domain::table_model::types::{
    ParameterBinding, Rows, SelectedRows, StoredSelection, Table,
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::service::to_bits;
use ryo_language::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell,
};
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::terms::service::term_symbol;
use ryo_language::domain::terms::types::{DeclarationKind, NameText, Symbol};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as Source, LiteralText, Operand, ParameterId, QueryLiteral,
    RepeatedRows, Test, WholeNumber,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::Arc;

/// One result-sharing, transport or fallback boundary.
#[derive(Clone, Copy)]
enum Scenario {
    /// Identical predicates and projection reuse one successful result.
    Reuse,
    /// A different projection shares the same physical match IDs.
    Projected,
    /// Zero-width output retains repeated row positions and their count.
    EmptyColumns,
    /// Refused cache retention still returns the complete local result.
    Budget,
    /// Incomplete provider columns retain ordinary execution.
    Ragged,
    /// A population outside the eligibility set bypasses compact execution.
    Ineligible,
    /// Referenced parameter values distinguish successful results.
    Bindings,
    /// A failed predicate neither admits a result nor consumes cache ownership.
    Error,
    /// A new physical generation starts without previous match IDs.
    Generation,
}
/// The optimization boundary exercised by one case.
type Input = Scenario;
/// One when all differential and ownership assertions hold.
type Value = usize;
/// Refusals from the ordinary typed evaluator.
type Failure = OperatingError;

/// A predicate and a duplicate output map, with only its referenced bound value.
fn program(value: i64) -> Arc<Selector> {
    Arc::new(Selector {
        relation: Relation::Function,
        identity: Arc::new(crate::domain::table_model::types::SelectionIdentity {
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Parameter(ParameterId(0)),
                origin: None,
            }],
            bindings: vec![ParameterBinding::Number(WholeNumber(value))],
        }),
        columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
        distinct: false,
    })
}

/// Exact ordinary operator execution remains the independent result oracle.
fn baseline(source: ColumnsView<'_>, program: &Selector) -> Result<Table, Failure> {
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &program.identity.bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    let operation = Operation::Select {
        from: Source::Relation(Relation::Function),
        comparisons: program.identity.comparisons.clone(),
        kept: if program.distinct {
            Vec::new()
        } else {
            program.columns.clone()
        },
    };
    let mut out = Table::default();
    compute(&operation, &[Rows::Stored(source)], &mut context, &mut out)?;
    if program.distinct {
        let mut projected = Table::default();
        compute(
            &Operation::Project {
                from: Source::Relation(program.relation),
                columns: program.columns.clone(),
                distinct: RepeatedRows::Dropped,
            },
            &[Rows::Computed(&out)],
            &mut context,
            &mut projected,
        )?;
        return Ok(projected);
    }
    Ok(out)
}

/// Compare compact and ordinary results before checking the requested cache boundary.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let columns: Vec<_> = (0_i64..3)
        .map(|column| {
            FactColumn(
                (0_i64..128)
                    .map(|row| {
                        to_bits(Cell::Number(Integer(if column == 0 {
                            row % 16
                        } else {
                            row * 10 + column
                        })))
                        .0
                    })
                    .collect(),
            )
        })
        .collect();
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let first = program(2);
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("generation");
    cache.selectors = vec![Arc::clone(&first), Arc::clone(&first), program(3)];
    crate::domain::stored_indexes::service::candidate_columns(&mut cache);
    if matches!(scenario, Scenario::Ineligible) {
        cache.eligible.clear();
    }
    if matches!(scenario, Scenario::Budget) {
        cache.limit = cache.bytes;
    }
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    if matches!(scenario, Scenario::Ragged | Scenario::Ineligible) {
        let input = if matches!(scenario, Scenario::Ragged) {
            ColumnsView {
                columns: &columns[..2],
                ..source
            }
        } else {
            source
        };
        assert!(select(&first, input, &mut context)?.is_none());
        return Ok(1);
    }
    if matches!(scenario, Scenario::EmptyColumns) {
        let rows = Rows::Selected(SelectedRows {
            source,
            selection: Arc::new(StoredSelection {
                relation: Relation::Function,
                rows: Arc::new(vec![3, 1, 3]),
                columns: Vec::new(),
                kinds: Vec::new(),
                identity: Arc::new(Default::default()),
            }),
        });
        let mut out = Table::default();
        materialize(&rows, &mut out, &mut Vec::new());
        assert_eq!((out.count.0, out.cells.len(), out.kinds.len()), (3, 0, 0));
        return Ok(1);
    }
    if matches!(scenario, Scenario::Error) {
        let mut bad = (*first).clone();
        Arc::make_mut(&mut bad.identity).bindings = vec![ParameterBinding::Tables(Vec::new())];
        assert!(matches!(
            select(&Arc::new(bad), source, &mut context),
            Err(OperatingError::WrongBinding)
        ));
        assert!(
            context
                .stored_indexes
                .as_ref()
                .unwrap()
                .selections
                .is_empty()
        );
    }
    let selected = select(&first, source, &mut context)?.expect("compact");
    let mut actual = Table::default();
    materialize(
        &Rows::Selected(selected.clone()),
        &mut actual,
        &mut Vec::new(),
    );
    assert_eq!(actual, baseline(source, &first)?);
    assert_eq!(actual.count.0, 8);
    let second = if matches!(scenario, Scenario::Projected) {
        let mut projected = (*first).clone();
        projected.columns = vec![ColumnIndex(2), ColumnIndex(2)];
        Arc::new(projected)
    } else if matches!(scenario, Scenario::Bindings) {
        program(3)
    } else {
        Arc::clone(&first)
    };
    let next = select(&second, source, &mut context)?.expect("compact replay");
    if matches!(scenario, Scenario::Reuse | Scenario::Projected) {
        assert_eq!(context.stored_indexes.as_ref().unwrap().builds, 1);
    }
    if matches!(scenario, Scenario::Reuse | Scenario::Projected) {
        assert!(Arc::ptr_eq(&selected.selection.rows, &next.selection.rows));
        let counters = &context.stored_indexes.as_ref().unwrap().selection_counts;
        assert_eq!(
            (counters.builds, counters.reuses, counters.scanned),
            (1, 1, 128)
        );
    }
    if matches!(scenario, Scenario::Budget) {
        assert!(
            context
                .stored_indexes
                .as_ref()
                .unwrap()
                .selections
                .is_empty()
        );
        assert_eq!(
            context
                .stored_indexes
                .as_ref()
                .unwrap()
                .selection_counts
                .reuses,
            0
        );
    }
    if matches!(scenario, Scenario::Bindings) {
        assert_ne!(selected.selection.rows, next.selection.rows);
    }
    if matches!(scenario, Scenario::Generation) {
        let changed = [
            FactColumn(vec![2]),
            FactColumn(vec![999]),
            FactColumn(vec![888]),
        ];
        let mut fresh = new(&[Relation::Function], &[], LIMIT).unwrap();
        let mut fresh_context = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: Some(&mut fresh),
            bindings: &[],
            symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo: context.memo,
            scratch: context.scratch,
        };
        let changed = select(
            &first,
            ColumnsView {
                columns: &changed,
                ..source
            },
            &mut fresh_context,
        )?
        .unwrap();
        assert_eq!(changed.selection.rows.len(), 1);
        assert!(!Arc::ptr_eq(
            &selected.selection.rows,
            &changed.selection.rows
        ));
    }
    Ok(1)
}

/// A closed term and a file-interned name can spell the same text. An equality
/// index over raw Symbol IDs must decline that ambiguous discriminator.
#[test]
fn symbol_discriminator_preserves_text_equality_with_term_aliases() -> Result<(), Failure> {
    let mut symbols = Symbols::default();
    let dynamic_class = intern_derived(&mut symbols, ByteSlice(b""), NameText("class"));
    let rare = intern_derived(&mut symbols, ByteSlice(b""), NameText("rare"));
    let class_term = term_symbol(DeclarationKind::Class);
    let method_term = term_symbol(DeclarationKind::Method);
    assert_ne!(dynamic_class, class_term);
    let mut kind = vec![to_bits(Cell::Symbol(method_term)).0; 128];
    for row in [0, 3, 17] {
        kind[row] = to_bits(Cell::Symbol(class_term)).0;
    }
    kind[2] = to_bits(Cell::Symbol(dynamic_class)).0;
    kind[4] = to_bits(Cell::Symbol(rare)).0;
    let columns = [
        FactColumn(kind),
        FactColumn(
            (0..128)
                .map(|row| to_bits(Cell::Number(Integer(row % 2))).0)
                .collect(),
        ),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol, CellKind::Number],
        columns: &columns,
    };
    let selector = |word: &str| {
        Arc::new(Selector {
            relation: Relation::Defines,
            identity: Arc::new(crate::domain::table_model::types::SelectionIdentity {
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(word.into()))),
                    origin: None,
                }],
                bindings: Vec::new(),
            }),
            columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
            distinct: false,
        })
    };
    let rare_selector = selector("rare");
    let class_selector = selector("class");
    let mut cache = new(&[Relation::Defines], &[], LIMIT).expect("generation");
    cache.selectors = vec![Arc::clone(&rare_selector), Arc::clone(&class_selector)];
    crate::domain::stored_indexes::service::candidate_columns(&mut cache);
    assert!(
        cache
            .discriminator_columns
            .iter()
            .any(|column| column.repeated)
    );
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let rare_rows = select(&rare_selector, source, &mut context)?.expect("indexed rare");
    assert_eq!(*rare_rows.selection.rows, [4]);
    let indexes = context.stored_indexes.as_deref().expect("cache");
    assert_eq!(indexes.entries.len(), 1, "unique name uses the index");
    let candidates_before = indexes.selection_counts.candidates;
    let scans_before = indexes.selection_counts.scanned;
    let class_rows = select(&class_selector, source, &mut context)?.expect("term alias scan");
    assert_eq!(*class_rows.selection.rows, [0, 2, 3, 17]);
    let indexes = context.stored_indexes.as_deref().expect("cache");
    assert_eq!(indexes.entries.len(), 1);
    assert_eq!(indexes.selection_counts.candidates, candidates_before);
    assert_eq!(indexes.selection_counts.scanned - scans_before, 128);
    let mut actual = Table::default();
    materialize(&Rows::Selected(class_rows), &mut actual, &mut Vec::new());
    drop(context);
    let mut ordinary = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut expected = Table::default();
    compute(
        &Operation::Select {
            from: Source::Relation(Relation::Defines),
            comparisons: class_selector.identity.comparisons.clone(),
            kept: class_selector.columns.clone(),
        },
        &[Rows::Stored(source)],
        &mut ordinary,
        &mut expected,
    )?;
    assert_eq!(actual, expected);
    assert_eq!(actual.count.0, 4);
    assert_eq!(
        &actual.cells[6..9],
        &actual.cells[9..12],
        "duplicate bag rows"
    );
    Ok(())
}

/// Distinct active literal demands share one physical scan, while each selector
/// retains the ordinary text-equality bag, order, and caller-owned error path.
#[test]
fn literal_fanout_routes_aliases_once_and_refuses_incomplete_inputs() -> Result<(), Failure> {
    let mut symbols = Symbols::default();
    let dynamic_class = intern_derived(&mut symbols, ByteSlice(b""), NameText("class"));
    let extras: Vec<_> = (0..7)
        .map(|number| {
            intern_derived(
                &mut symbols,
                ByteSlice(b""),
                NameText(&format!("word{number}")),
            )
        })
        .collect();
    let mut bits = vec![to_bits(Cell::Symbol(term_symbol(DeclarationKind::Method))).0; 128];
    for row in [1, 4, 8] {
        bits[row] = to_bits(Cell::Symbol(term_symbol(DeclarationKind::Class))).0;
    }
    bits[5] = to_bits(Cell::Symbol(dynamic_class)).0;
    bits[127] = to_bits(Cell::Symbol(Symbol(u32::MAX))).0;
    for (number, symbol) in extras.iter().enumerate() {
        bits[20 + number * 2] = to_bits(Cell::Symbol(*symbol)).0;
    }
    let columns = [FactColumn(bits)];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let selector = |text: &str| {
        Arc::new(Selector {
            relation: Relation::Defines,
            identity: Arc::new(crate::domain::table_model::types::SelectionIdentity {
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
                    origin: None,
                }],
                bindings: Vec::new(),
            }),
            columns: vec![ColumnIndex(0), ColumnIndex(0)],
            distinct: false,
        })
    };
    let mut selectors = vec![selector("class")];
    selectors.extend((0..7).map(|number| selector(&format!("word{number}"))));
    let mut cache = new(&[Relation::Defines], &[], LIMIT).expect("generation");
    cache.selectors = selectors.clone();
    crate::domain::stored_indexes::service::candidate_columns(&mut cache);
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let inactive = selector("absent");
    assert!(
        select(&inactive, source, &mut context)?
            .expect("ordinary absent-literal fallback")
            .selection
            .rows
            .is_empty()
    );
    assert_eq!(
        context.stored_indexes.as_ref().unwrap().fanouts.capacity(),
        0
    );
    let mut actual = Vec::new();
    for selector in &selectors {
        let selected = select(selector, source, &mut context)?.expect("compact selection");
        let mut table = Table::default();
        materialize(&Rows::Selected(selected), &mut table, &mut Vec::new());
        actual.push(table);
    }
    let indexes = context.stored_indexes.as_ref().expect("cache");
    assert_eq!(indexes.fanouts.len(), 1);
    assert_eq!(indexes.selection_counts.scanned, 128, "one source pass");
    assert_eq!(indexes.entries.len(), 0, "no whole-column index");
    assert_eq!(actual[0].count.0, 4);
    let bad = Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Parameter(ParameterId(0)),
        origin: None,
    };
    let mut error_selector = (*selectors[0]).clone();
    error_selector.identity = Arc::new(crate::domain::table_model::types::SelectionIdentity {
        comparisons: vec![selectors[0].identity.comparisons[0].clone(), bad.clone()],
        bindings: vec![ParameterBinding::Tables(Vec::new())],
    });
    assert!(matches!(
        select(&Arc::new(error_selector), source, &mut context),
        Err(OperatingError::WrongBinding)
    ));
    let mut absent_then_error = (*selectors[0]).clone();
    absent_then_error.identity = Arc::new(crate::domain::table_model::types::SelectionIdentity {
        comparisons: vec![selector("absent").identity.comparisons[0].clone(), bad],
        bindings: vec![ParameterBinding::Tables(Vec::new())],
    });
    assert!(
        select(&Arc::new(absent_then_error), source, &mut context)?
            .expect("empty complete")
            .selection
            .rows
            .is_empty()
    );
    drop(context);
    for (selector, result) in selectors.iter().zip(actual) {
        let mut ordinary = OperatorContext {
            prepared_kinds: None,
            prepared_matchers: None,
            stored_indexes: None,
            bindings: &selector.identity.bindings,
            symbols: (&mut symbols).into(),
            source: SourceView(b""),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let mut expected = Table::default();
        compute(
            &Operation::Select {
                from: Source::Relation(Relation::Defines),
                comparisons: selector.identity.comparisons.clone(),
                kept: selector.columns.clone(),
            },
            &[Rows::Stored(source)],
            &mut ordinary,
            &mut expected,
        )?;
        assert_eq!(result, expected);
    }
    let mut constrained = new(&[Relation::Defines], &[], LIMIT).expect("generation");
    constrained.selectors = selectors.clone();
    constrained.limit = constrained.bytes;
    let mut budget_context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut constrained),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    assert_eq!(
        select(&selectors[0], source, &mut budget_context)?
            .expect("fallback")
            .selection
            .rows
            .len(),
        4
    );
    assert!(
        budget_context
            .stored_indexes
            .as_ref()
            .unwrap()
            .fanouts
            .is_empty()
    );
    drop(budget_context);
    let empty_columns = [FactColumn(Vec::new())];
    let empty = ColumnsView {
        columns: &empty_columns,
        ..source
    };
    let mut complete_empty = new(&[Relation::Defines], &[], LIMIT).expect("generation");
    let mut empty_context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut complete_empty),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    assert!(
        select(&selectors[0], empty, &mut empty_context)?
            .expect("complete empty")
            .selection
            .rows
            .is_empty()
    );
    assert!(
        empty_context
            .stored_indexes
            .as_ref()
            .unwrap()
            .fanouts
            .is_empty()
    );
    assert!(
        select(
            &selectors[0],
            ColumnsView {
                columns: &[],
                ..source
            },
            &mut empty_context
        )?
        .is_none()
    );
    drop(empty_context);
    let partial = Partial {
        relation: Relation::Defines,
        cause: PartialCause("missing syntax".into()),
        lacking: Lacking::Any,
        at: None,
    };
    let mut incomplete = new(&[Relation::Defines], &[partial], LIMIT).expect("generation");
    let mut partial_context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut incomplete),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    assert!(select(&selectors[0], source, &mut partial_context)?.is_none());
    assert!(
        partial_context
            .stored_indexes
            .as_ref()
            .unwrap()
            .fanouts
            .is_empty()
    );
    Ok(())
}

/// A spelling absent from both symbol registries cannot match a nonempty
/// literal, but a preceding comparison can still fail before that equality.
#[test]
fn absent_symbol_literal_preserves_ordered_errors() -> Result<(), Failure> {
    let mut symbols = Symbols::default();
    let present = intern_derived(&mut symbols, ByteSlice(b""), NameText("present"));
    let columns = [FactColumn(vec![to_bits(Cell::Symbol(present)).0; 64])];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let absent = Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("absent".into()))),
        origin: None,
    };
    let bad = Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(0)),
        right: Operand::Parameter(ParameterId(0)),
        origin: None,
    };
    let selector = |comparisons| {
        Arc::new(Selector {
            relation: Relation::Defines,
            identity: Arc::new(crate::domain::table_model::types::SelectionIdentity {
                comparisons,
                bindings: vec![ParameterBinding::Tables(Vec::new())],
            }),
            columns: vec![ColumnIndex(0), ColumnIndex(0)],
            distinct: false,
        })
    };
    let first = selector(vec![absent.clone(), bad.clone()]);
    let reversed = selector(vec![bad, absent]);
    let mut cache = new(&[Relation::Defines], &[], LIMIT).expect("generation");
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let selected = select(&first, source, &mut context)?.expect("compact result");
    assert!(selected.selection.rows.is_empty());
    let mut actual = Table::default();
    materialize(&Rows::Selected(selected), &mut actual, &mut Vec::new());
    assert_eq!(
        context
            .stored_indexes
            .as_ref()
            .unwrap()
            .selection_counts
            .scanned,
        0
    );
    assert!(context.stored_indexes.as_ref().unwrap().entries.is_empty());
    assert!(matches!(
        select(&reversed, source, &mut context),
        Err(OperatingError::WrongBinding)
    ));
    drop(context);
    let mut ordinary = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &first.identity.bindings,
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut expected = Table::default();
    compute(
        &Operation::Select {
            from: Source::Relation(Relation::Defines),
            comparisons: first.identity.comparisons.clone(),
            kept: first.columns.clone(),
        },
        &[Rows::Stored(source)],
        &mut ordinary,
        &mut expected,
    )?;
    assert_eq!(actual, expected);
    assert_eq!(actual.count.0, 0);
    assert!(matches!(
        compute(
            &Operation::Select {
                from: Source::Relation(Relation::Defines),
                comparisons: reversed.identity.comparisons.clone(),
                kept: reversed.columns.clone(),
            },
            &[Rows::Stored(source)],
            &mut ordinary,
            &mut Table::default(),
        ),
        Err(OperatingError::WrongBinding)
    ));
    Ok(())
}

/// Invalid Symbol IDs can decode as empty text in ordinary evaluation. They
/// must not be ruled out by an absent empty literal.
#[test]
fn empty_literal_preserves_malformed_symbol_fallback() -> Result<(), Failure> {
    let mut symbols = Symbols::default();
    let invalid = Symbol(u32::MAX);
    let columns = [FactColumn(vec![to_bits(Cell::Symbol(invalid)).0; 3])];
    let source = ColumnsView {
        kinds: &[CellKind::Symbol],
        columns: &columns,
    };
    let selector = Arc::new(Selector {
        relation: Relation::Defines,
        identity: Arc::new(crate::domain::table_model::types::SelectionIdentity {
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("".into()))),
                origin: None,
            }],
            bindings: Vec::new(),
        }),
        columns: vec![ColumnIndex(0), ColumnIndex(0)],
        distinct: false,
    });
    let mut cache = new(&[Relation::Defines], &[], LIMIT).expect("generation");
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let selected = select(&selector, source, &mut context)?.expect("compact result");
    assert_eq!(*selected.selection.rows, [0, 1, 2]);
    assert_eq!(
        context
            .stored_indexes
            .as_ref()
            .unwrap()
            .selection_counts
            .scanned,
        3
    );
    let mut actual = Table::default();
    materialize(&Rows::Selected(selected), &mut actual, &mut Vec::new());
    drop(context);
    let mut ordinary = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut expected = Table::default();
    compute(
        &Operation::Select {
            from: Source::Relation(Relation::Defines),
            comparisons: selector.identity.comparisons.clone(),
            kept: selector.columns.clone(),
        },
        &[Rows::Stored(source)],
        &mut ordinary,
        &mut expected,
    )?;
    assert_eq!(actual, expected);
    assert_eq!(actual.count.0, 3);
    Ok(())
}

/// Registered success, refusal and generation boundaries of compact selection.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordered_duplicate_matches_share_one_index_and_selection"),
        word: Word::Succeeds,
        input: || Scenario::Reuse,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("different_duplicate_projections_share_match_ids"),
        word: Word::Succeeds,
        input: || Scenario::Projected,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("zero_width_selected_rows_keep_count_and_repeated_positions"),
        word: Word::Succeeds,
        input: || Scenario::EmptyColumns,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("cache_budget_refusal_keeps_exact_local_result"),
        word: Word::Succeeds,
        input: || Scenario::Budget,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("ragged_views_fall_back_without_cached_rows"),
        word: Word::Succeeds,
        input: || Scenario::Ragged,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("ineligible_population_is_not_selected"),
        word: Word::Succeeds,
        input: || Scenario::Ineligible,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("changed_referenced_binding_separates_results"),
        word: Word::Succeeds,
        input: || Scenario::Bindings,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("failed_selection_restores_cache_and_admits_no_result"),
        word: Word::Succeeds,
        input: || Scenario::Error,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("next_generation_cannot_reuse_old_ids"),
        word: Word::Succeeds,
        input: || Scenario::Generation,
        expect: Expect::Value(1),
    },
];

/// Execute every registered compact-selection differential case.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// Entries append into spare owned capacity instead of moving every retained entry.
#[test]
fn retained_selections_keep_spare_vector_capacity() -> Result<(), Failure> {
    let columns: Vec<_> = (0_i64..3)
        .map(|column| {
            FactColumn(
                (0_i64..4)
                    .map(|row| to_bits(Cell::Number(Integer(row + column))).0)
                    .collect(),
            )
        })
        .collect();
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let (first, second) = (program(2), program(3));
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("generation");
    cache.selectors = vec![
        Arc::clone(&first),
        Arc::clone(&first),
        Arc::clone(&second),
        Arc::clone(&second),
    ];
    cache
        .selections
        .try_reserve_exact(8)
        .expect("test spare capacity");
    let storage = cache.selections.as_ptr();
    let before = cache.bytes;
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let selected = select(&first, source, &mut context)?.expect("first result");
    let selected_next = select(&second, source, &mut context)?.expect("second result");
    assert_ne!(selected.selection.rows, selected_next.selection.rows);
    let retained = context.stored_indexes.as_ref().expect("cache restored");
    assert_eq!(retained.selections.len(), 2);
    assert_eq!(retained.selections.as_ptr(), storage);
    assert_eq!(
        retained.bytes,
        before + super::service::RETAINED_SELECTION_RESERVATION
    );
    Ok(())
}

/// A result larger than the fixed selection allowance stays local to its call.
#[test]
fn oversized_selection_refuses_retention_without_charging_the_cache() {
    let program = program(2);
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("generation");
    cache.selectors = vec![Arc::clone(&program), Arc::clone(&program)];
    let before = cache.bytes;
    let rows = Arc::new(Vec::with_capacity(
        super::service::MAX_RETAINED_POSITION_CAPACITY + 1,
    ));
    let columns = [
        FactColumn::default(),
        FactColumn::default(),
        FactColumn::default(),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let _admission = super::service::admit(&mut cache, &program, source, 0, &rows);
    assert!(cache.selections.is_empty());
    assert_eq!(cache.selection_counts.refusals, 1);
    assert_eq!(cache.bytes, before);
}

/// Distinct projections keep independent first representatives while sharing only predicate matches.
#[test]
fn distinct_projections_preserve_rows_and_predicate_cache() -> Result<(), Failure> {
    let columns = [
        FactColumn(vec![7, 7, 7, 7, 7, 8]),
        // Unequal bit encodings of the same typed flag must deduplicate.
        FactColumn(vec![1, 2, 0, 7, 0, 1]),
        FactColumn(vec![10, 10, 20, 11, 20, 10]),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
        columns: &columns,
    };
    let mut first = (*program(7)).clone();
    first.columns = vec![ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)];
    first.distinct = true;
    let first = Arc::new(first);
    let mut second = (*first).clone();
    second.columns = vec![ColumnIndex(1)];
    let second = Arc::new(second);
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("generation");
    cache.selectors = vec![Arc::clone(&first), Arc::clone(&second)];
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    for (program, expected) in [(&first, vec![0, 2, 3]), (&second, vec![0, 2])] {
        let actual = select(program, source, &mut context)?.expect("compact distinct");
        assert_eq!(*actual.selection.rows, expected);
        let mut table = Table::default();
        materialize(&Rows::Selected(actual), &mut table, &mut Vec::new());
        assert_eq!(table, baseline(source, program)?);
    }
    let mut bag = (*first).clone();
    bag.distinct = false;
    let bag = select(&Arc::new(bag), source, &mut context)?.unwrap();
    assert_eq!(*bag.selection.rows, [0, 1, 2, 3, 4]);
    assert_eq!(context.stored_indexes.as_ref().unwrap().selections.len(), 1);
    assert_eq!(
        context.stored_indexes.as_ref().unwrap().selections[0]
            .rows
            .as_slice(),
        [0, 1, 2, 3, 4]
    );

    let mut zero = (*first).clone();
    zero.columns.clear();
    let zero = Arc::new(zero);
    let selected = select(&zero, source, &mut context)?.unwrap();
    assert_eq!(*selected.selection.rows, [0]);
    let mut table = Table::default();
    materialize(&Rows::Selected(selected), &mut table, &mut Vec::new());
    assert_eq!(table, baseline(source, &zero)?);
    assert_eq!((table.count.0, table.cells.len()), (1, 0));
    let mut absent = (*zero).clone();
    Arc::make_mut(&mut absent.identity).bindings = vec![ParameterBinding::Number(WholeNumber(999))];
    assert!(
        select(&Arc::new(absent), source, &mut context)?
            .unwrap()
            .selection
            .rows
            .is_empty()
    );

    let empty_columns = [
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
    ];
    let mut fresh = new(&[Relation::Function], &[], LIMIT).unwrap();
    context.stored_indexes = Some(&mut fresh);
    let empty = ColumnsView {
        columns: &empty_columns,
        ..source
    };
    assert!(
        select(&zero, empty, &mut context)?
            .unwrap()
            .selection
            .rows
            .is_empty()
    );
    let changed_columns = [
        FactColumn(vec![7]),
        FactColumn(vec![0]),
        FactColumn(vec![20]),
    ];
    let mut next_file = new(&[Relation::Function], &[], LIMIT).unwrap();
    context.stored_indexes = Some(&mut next_file);
    let changed = ColumnsView {
        columns: &changed_columns,
        ..source
    };
    let actual = select(&first, changed, &mut context)?.unwrap();
    assert_eq!(*actual.selection.rows, [0]);
    let mut table = Table::default();
    materialize(&Rows::Selected(actual), &mut table, &mut Vec::new());
    assert_eq!(table, baseline(changed, &first)?);

    let constant_hash = |_: ColumnsView<'_>, _: &[ColumnIndex], _: u32| Some(0);
    assert_eq!(
        super::service::representatives(source, &first.columns, &[0, 1, 2, 3, 4], constant_hash),
        Some(vec![0, 2, 3]),
    );
    assert_eq!(
        super::service::representatives(source, &first.columns, &[0, 99], constant_hash),
        None,
    );
    assert_eq!(
        super::service::representatives(source, &[ColumnIndex(99)], &[0], constant_hash),
        None,
    );
    Ok(())
}
