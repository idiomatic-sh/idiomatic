//! The `stored_selection` service's cases, on values alone.

use super::service::select;
use super::types::Selector;
use crate::cases::{Case, CaseName, Expect, Word, run};
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
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as Source, Operand, ParameterId, Test, WholeNumber,
};
use ryo_values::domain::plans::types::Operation;
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
        comparisons: vec![Comparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Parameter(ParameterId(0)),
            origin: None,
        }],
        bindings: vec![ParameterBinding::Number(WholeNumber(value))],
        columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
    })
}

/// Exact ordinary operator execution remains the independent result oracle.
fn baseline(source: ColumnsView<'_>, program: &Selector) -> Result<Table, Failure> {
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        stored_indexes: None,
        bindings: &program.bindings,
        symbols: &mut symbols,
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    let operation = Operation::Select {
        from: Source::Relation(Relation::Function),
        comparisons: program.comparisons.clone(),
        kept: program.columns.clone(),
    };
    let mut out = Table::default();
    compute(&operation, &[Rows::Stored(source)], &mut context, &mut out)?;
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
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: &mut symbols,
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
            }),
        });
        let mut out = Table::default();
        materialize(&rows, &mut out, &mut Vec::new());
        assert_eq!((out.count.0, out.cells.len(), out.kinds.len()), (3, 0, 0));
        return Ok(1);
    }
    if matches!(scenario, Scenario::Error) {
        let mut bad = (*first).clone();
        bad.bindings = vec![ParameterBinding::Tables(Vec::new())];
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
            stored_indexes: Some(&mut fresh),
            bindings: &[],
            symbols: context.symbols,
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
