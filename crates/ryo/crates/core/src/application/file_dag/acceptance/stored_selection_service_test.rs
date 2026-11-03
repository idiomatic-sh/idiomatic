//! The `stored_selection` service's cases, on values alone.

use crate::application::file_dag::cross_execution::compute;
use crate::application::file_dag::derived::owner::selection_fixture::SelectionFixture;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::coverage::types::{Lacking, Partial};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::stored_selection::types::Selector;
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

use crate::domain::stored_selection::fixtures::program;

/// Exact ordinary operator execution remains the independent result oracle.
fn baseline(source: ColumnsView<'_>, program: &Selector) -> Result<Table, Failure> {
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
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
    let alternate = program(3);
    let second = if matches!(scenario, Scenario::Projected) {
        let mut projected = (*first).clone();
        projected.columns = vec![ColumnIndex(2), ColumnIndex(2)];
        Arc::new(projected)
    } else if matches!(scenario, Scenario::Bindings) {
        (alternate).clone()
    } else {
        (first).clone()
    };
    let mut bad = (*first).clone();
    Arc::make_mut(&mut bad.identity).bindings = vec![ParameterBinding::Tables(Vec::new())];
    let bad = Arc::new(bad);
    let active = vec![first.clone(), first.clone(), alternate.clone()];
    let authored = active
        .iter()
        .cloned()
        .chain([second.clone(), bad.clone()])
        .collect::<Vec<_>>();
    let fixture =
        SelectionFixture::new(&[Relation::Function], &[], active, authored, source.kinds)?;
    if matches!(scenario, Scenario::Ineligible) {
        fixture.ineligible();
    }
    if matches!(scenario, Scenario::Budget) {
        fixture.limit_to_metadata();
    }
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
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
        assert!(fixture.select(&first, input, &mut context)?.is_none());
        return Ok(1);
    }
    if matches!(scenario, Scenario::EmptyColumns) {
        // Pure materialization control for zero-width bags, including repeated positions.
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
        assert!(matches!(
            fixture.select(&bad, source, &mut context),
            Err(OperatingError::WrongBinding)
        ));
        assert_eq!(fixture.snapshot(None).selections, 0);
    }
    let selected = fixture
        .select(&first, source, &mut context)?
        .expect("compact");
    let mut actual = Table::default();
    materialize(
        &Rows::Selected(selected.rows.clone()),
        &mut actual,
        &mut Vec::new(),
    );
    assert_eq!(actual, baseline(source, &first)?);
    assert_eq!(actual.count.0, 8);
    let next = fixture
        .select(&second, source, &mut context)?
        .expect("compact replay");
    if matches!(scenario, Scenario::Reuse | Scenario::Projected) {
        let snapshot = fixture.snapshot(None);
        assert_eq!(snapshot.index_builds, 1);
        assert!(Arc::ptr_eq(&selected.selection.rows, &next.selection.rows));
        assert_eq!(
            (
                snapshot.counts.builds,
                snapshot.counts.reuses,
                snapshot.counts.scanned
            ),
            (1, 1, 128)
        );
    }
    if matches!(scenario, Scenario::Budget) {
        let snapshot = fixture.snapshot(None);
        assert_eq!(snapshot.bytes, snapshot.limit);
        assert_eq!(snapshot.selections, 0);
        assert_eq!(snapshot.optional_bytes, 0);
        assert_eq!(
            snapshot.counts.reuses, 1,
            "live publications share without optional retention"
        );
        assert!(Arc::ptr_eq(&selected.selection.rows, &next.selection.rows));
        let old = Arc::downgrade(&selected.selection.rows);
        let builds = snapshot.counts.builds;
        drop(selected);
        drop(next);
        assert!(
            old.upgrade().is_none(),
            "last consumer releases uncached positions"
        );
        let fresh = fixture
            .select(&first, source, &mut context)?
            .expect("valid fresh required view");
        let mut fresh_table = Table::default();
        materialize(
            &Rows::Selected(fresh.rows.clone()),
            &mut fresh_table,
            &mut Vec::new(),
        );
        assert_eq!(fresh_table, baseline(source, &first)?);
        let after = fixture.snapshot(None);
        assert_eq!(after.counts.builds, builds + 1);
        assert_eq!(after.optional_bytes, 0);
        assert_eq!(after.selections, 0);
        return Ok(1);
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
        let active = vec![first.clone(), first.clone(), alternate.clone()];
        let fresh = SelectionFixture::new(
            &[Relation::Function],
            &[],
            active.clone(),
            active.into_iter().chain([second.clone(), bad.clone()]),
            source.kinds,
        )?;
        let changed = fresh
            .select(
                &first,
                ColumnsView {
                    columns: &changed,
                    ..source
                },
                &mut context,
            )?
            .expect("new generation");
        assert_eq!(changed.selection.rows.len(), 1);
        assert!(!Arc::ptr_eq(
            &selected.selection.rows,
            &changed.selection.rows
        ));
        assert!(!selected.publication.same_record(&changed.publication));
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
    let active = vec![rare_selector.clone(), class_selector.clone()];
    let fixture = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        active.clone(),
        active,
        source.kinds,
    )?;
    assert!(fixture.snapshot(None).repeated_discriminator);
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let rare_rows = fixture
        .select(&rare_selector, source, &mut context)?
        .expect("indexed rare");
    assert_eq!(*rare_rows.selection.rows, [4]);
    let indexes = fixture.snapshot(None);
    assert_eq!(indexes.entries, 1, "unique name uses the index");
    let candidates_before = indexes.counts.candidates;
    let scans_before = indexes.counts.scanned;
    let class_rows = fixture
        .select(&class_selector, source, &mut context)?
        .expect("term alias scan");
    assert_eq!(*class_rows.selection.rows, [0, 2, 3, 17]);
    let indexes = fixture.snapshot(None);
    assert_eq!(indexes.entries, 1);
    assert_eq!(indexes.counts.candidates, candidates_before);
    assert_eq!(indexes.counts.scanned - scans_before, 128);
    let mut actual = Table::default();
    materialize(
        &Rows::Selected(class_rows.rows.clone()),
        &mut actual,
        &mut Vec::new(),
    );
    drop(context);
    let mut ordinary = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
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
    let inactive = selector("absent");
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
    let error_selector = Arc::new(error_selector);
    let mut absent_then_error = (*selectors[0]).clone();
    absent_then_error.identity = Arc::new(crate::domain::table_model::types::SelectionIdentity {
        comparisons: vec![selector("absent").identity.comparisons[0].clone(), bad],
        bindings: vec![ParameterBinding::Tables(Vec::new())],
    });
    let absent_then_error = Arc::new(absent_then_error);
    let authored: Vec<_> = selectors
        .iter()
        .cloned()
        .chain([
            inactive.clone(),
            error_selector.clone(),
            absent_then_error.clone(),
        ])
        .collect();
    let fixture = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        selectors.clone(),
        authored.clone(),
        source.kinds,
    )?;
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    assert!(
        fixture
            .select(&inactive, source, &mut context)?
            .expect("ordinary absent-literal fallback")
            .selection
            .rows
            .is_empty()
    );
    assert_eq!(fixture.snapshot(None).fanouts_capacity, 0);
    let mut actual = Vec::new();
    for selector in &selectors {
        let selected = fixture
            .select(selector, source, &mut context)?
            .expect("compact selection");
        let mut table = Table::default();
        materialize(
            &Rows::Selected(selected.rows.clone()),
            &mut table,
            &mut Vec::new(),
        );
        actual.push(table);
    }
    let indexes = fixture.snapshot(None);
    assert_eq!(indexes.fanouts, 1);
    assert_eq!(
        indexes.retained_positions,
        indexes.retained_positions_capacity
    );
    assert_eq!(indexes.counts.scanned, 128, "one source pass");
    assert_eq!(indexes.entries, 0, "no whole-column index");
    assert_eq!(actual[0].count.0, 4);
    assert!(matches!(
        fixture.select(&error_selector, source, &mut context),
        Err(OperatingError::WrongBinding)
    ));
    assert!(
        fixture
            .select(&absent_then_error, source, &mut context)?
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
    let constrained = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        selectors.clone(),
        authored.clone(),
        source.kinds,
    )?;
    constrained.limit_to_metadata();
    let mut budget_context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    assert_eq!(
        constrained
            .select(&selectors[0], source, &mut budget_context)?
            .expect("fallback")
            .selection
            .rows
            .len(),
        4
    );
    assert_eq!(constrained.snapshot(None).fanouts, 0);
    drop(budget_context);
    let empty_columns = [FactColumn(Vec::new())];
    let empty = ColumnsView {
        columns: &empty_columns,
        ..source
    };
    let complete_empty = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        selectors.clone(),
        authored.clone(),
        source.kinds,
    )?;
    let mut empty_context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    assert!(
        complete_empty
            .select(&selectors[0], empty, &mut empty_context)?
            .expect("complete empty")
            .selection
            .rows
            .is_empty()
    );
    assert_eq!(complete_empty.snapshot(None).fanouts, 0);
    assert!(
        complete_empty
            .select(
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
    let incomplete = SelectionFixture::new(
        &[Relation::Defines],
        &[partial],
        selectors.clone(),
        authored,
        source.kinds,
    )?;
    let mut partial_context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    assert!(
        incomplete
            .select(&selectors[0], source, &mut partial_context)?
            .is_none()
    );
    assert_eq!(incomplete.snapshot(None).fanouts, 0);
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
    let active = vec![first.clone(), reversed.clone()];
    let fixture = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        active.clone(),
        active,
        source.kinds,
    )?;
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let selected = fixture
        .select(&first, source, &mut context)?
        .expect("compact result");
    assert!(selected.selection.rows.is_empty());
    let mut actual = Table::default();
    materialize(
        &Rows::Selected(selected.rows.clone()),
        &mut actual,
        &mut Vec::new(),
    );
    assert_eq!(fixture.snapshot(None).counts.scanned, 0);
    assert_eq!(fixture.snapshot(None).entries, 0);
    assert!(matches!(
        fixture.select(&reversed, source, &mut context),
        Err(OperatingError::WrongBinding)
    ));
    drop(context);
    let mut ordinary = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
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
    let fixture = SelectionFixture::new(
        &[Relation::Defines],
        &[],
        vec![selector.clone()],
        [selector.clone()],
        source.kinds,
    )?;
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let selected = fixture
        .select(&selector, source, &mut context)?
        .expect("compact result");
    assert_eq!(*selected.selection.rows, [0, 1, 2]);
    assert_eq!(fixture.snapshot(None).counts.scanned, 3);
    let mut actual = Table::default();
    materialize(
        &Rows::Selected(selected.rows.clone()),
        &mut actual,
        &mut Vec::new(),
    );
    drop(context);
    let mut ordinary = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
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
    let mut bag = (*first).clone();
    bag.distinct = false;
    let bag = Arc::new(bag);
    let mut zero = (*first).clone();
    zero.columns.clear();
    let zero = Arc::new(zero);
    let mut absent = (*zero).clone();
    Arc::make_mut(&mut absent.identity).bindings = vec![ParameterBinding::Number(WholeNumber(999))];
    let absent = Arc::new(absent);
    let authored = vec![
        first.clone(),
        second.clone(),
        bag.clone(),
        zero.clone(),
        absent.clone(),
    ];
    let active = vec![first.clone(), second.clone()];
    let fixture = SelectionFixture::new(
        &[Relation::Function],
        &[],
        active.clone(),
        authored.clone(),
        source.kinds,
    )?;
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    for (program, expected) in [(&first, vec![0, 2, 3]), (&second, vec![0, 2])] {
        let actual = fixture
            .select(program, source, &mut context)?
            .expect("compact distinct");
        assert_eq!(*actual.selection.rows, expected);
        let mut table = Table::default();
        materialize(
            &Rows::Selected(actual.rows.clone()),
            &mut table,
            &mut Vec::new(),
        );
        assert_eq!(table, baseline(source, program)?);
    }
    let bag_result = fixture
        .select(&bag, source, &mut context)?
        .expect("bag result");
    assert_eq!(*bag_result.selection.rows, [0, 1, 2, 3, 4]);
    let retained = fixture.snapshot(None);
    assert_eq!(retained.selections, 1);
    assert_eq!(
        retained.first_retained_rows_address,
        Some(Arc::as_ptr(&bag_result.selection.rows) as usize)
    );
    let selected = fixture
        .select(&zero, source, &mut context)?
        .expect("zero-width result");
    assert_eq!(*selected.selection.rows, [0]);
    let mut table = Table::default();
    materialize(
        &Rows::Selected(selected.rows.clone()),
        &mut table,
        &mut Vec::new(),
    );
    assert_eq!(table, baseline(source, &zero)?);
    assert_eq!((table.count.0, table.cells.len()), (1, 0));
    assert!(
        fixture
            .select(&absent, source, &mut context)?
            .expect("absent")
            .selection
            .rows
            .is_empty()
    );
    let empty_columns = [
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
    ];
    let fresh = SelectionFixture::new(
        &[Relation::Function],
        &[],
        active.clone(),
        authored.clone(),
        source.kinds,
    )?;
    let empty = ColumnsView {
        columns: &empty_columns,
        ..source
    };
    assert!(
        fresh
            .select(&zero, empty, &mut context)?
            .expect("empty file")
            .selection
            .rows
            .is_empty()
    );
    let changed_columns = [
        FactColumn(vec![7]),
        FactColumn(vec![0]),
        FactColumn(vec![20]),
    ];
    let next_file =
        SelectionFixture::new(&[Relation::Function], &[], active, authored, source.kinds)?;
    assert_eq!(
        next_file.snapshot(None).retained_positions,
        0,
        "a new generation retains no prior positions"
    );
    let changed = ColumnsView {
        columns: &changed_columns,
        ..source
    };
    let actual = next_file
        .select(&first, changed, &mut context)?
        .expect("next file");
    assert_eq!(*actual.selection.rows, [0]);
    let mut table = Table::default();
    materialize(
        &Rows::Selected(actual.rows.clone()),
        &mut table,
        &mut Vec::new(),
    );
    assert_eq!(table, baseline(changed, &first)?);
    Ok(())
}
