//! Generic stored-selection cache policy and independent public-owner output controls.
//! Synthetic schemas below parameterize the private generic cache contract;
//! they are never supplied as catalog providers to the public execution owner.

use crate::application::file_dag::derived::owner::selection::fixtures::SelectionFixture;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::stored_selection::fixtures::{expected_columns, program};
use crate::domain::table_model::service::materialize;
use crate::domain::table_model::types::{Rows, SelectedRows, StoredSelection};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::Table;
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
    /// Unequal column lengths retain readable ordinary fallback.
    Ragged,
    /// A schema width unlike the supplied physical columns is rejected.
    Malformed,
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
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        matcher: crate::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    if matches!(
        scenario,
        Scenario::Ragged | Scenario::Malformed | Scenario::Ineligible
    ) {
        let mut ragged_columns = columns
            .iter()
            .map(|column| FactColumn(column.0.clone()))
            .collect::<Vec<_>>();
        ragged_columns[1].0.pop();
        let input = match scenario {
            Scenario::Ragged => ColumnsView {
                columns: &ragged_columns,
                ..source
            },
            Scenario::Malformed => ColumnsView {
                columns: &columns[..2],
                ..source
            },
            _ => source,
        };
        let selected = fixture.select(&first, input, &mut context);
        assert_eq!(
            fixture.owner().checks.get(),
            crate::application::file_dag::relations::types::CheckCount(1),
            "the actual owner inspects the backing before declining optional selection"
        );
        assert_eq!(fixture.snapshot(None).counts.builds, 0);
        if matches!(scenario, Scenario::Malformed) {
            return match selected {
                Err(error) => Err(error),
                Ok(_) => panic!("schema/column-count mismatch must be refused by owner admission"),
            };
        }
        assert!(selected?.is_none());
        return Ok(1);
    }
    if matches!(scenario, Scenario::EmptyColumns) {
        // Pure materialization control for zero-width bags, including repeated positions.
        let rows = Rows::Selected(SelectedRows {
            source,
            selection: Arc::new(StoredSelection {
                relation: Relation::Function,
                rows: crate::domain::stored_selection::types::SelectedPositions(Arc::new(vec![3, 1, 3])),
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
    assert_eq!(
        actual,
        expected_columns(
            source,
            &first.columns,
            &(0..8).map(|n| 2 + n * 16).collect::<Vec<_>>()
        )
    );
    assert_eq!(actual.count.0, 8);
    let next = fixture
        .select(&second, source, &mut context)?
        .expect("compact replay");
    if matches!(scenario, Scenario::Reuse | Scenario::Projected) {
        let snapshot = fixture.snapshot(None);
        assert_eq!(snapshot.index_builds, 1);
        assert!(Arc::ptr_eq(&selected.selection.rows.0, &next.selection.rows.0));
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
        assert!(Arc::ptr_eq(&selected.selection.rows.0, &next.selection.rows.0));
        let old = Arc::downgrade(&selected.selection.rows.0);
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
        assert_eq!(
            fresh_table,
            expected_columns(
                source,
                &first.columns,
                &(0..8).map(|n| 2 + n * 16).collect::<Vec<_>>()
            )
        );
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
            &selected.selection.rows.0,
            &changed.selection.rows.0
        ));
        assert_eq!(selected.publication.same_record(&changed.publication), crate::application::file_dag::relations::types::Sameness::Different);
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
        name: CaseName("schema_width_mismatch_is_refused_before_selection"),
        word: Word::Refuses,
        input: || Scenario::Malformed,
        expect: Expect::Error(OperatingError::ShapeMismatch),
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

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
