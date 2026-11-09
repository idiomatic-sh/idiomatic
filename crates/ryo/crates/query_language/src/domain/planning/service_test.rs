//! Domain behavior from typed values, independent of parsing and adapters.
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{PlannedColumn, Uniqueness};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

fn frontier_keeps_only_file_to_project_dependencies() {
    let steps = vec![
        plans::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
            operation: plans::Operation::Select {
                from: parts::Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![],
            },
        },
        plans::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::CrossFile,
            operation: plans::Operation::Union {
                left: parts::Input::Step(parts::StepId(0)),
                right: parts::Input::Relation(Relation::SyntaxChild),
            },
        },
        plans::PlanStep {
            scope: ryo_values::domain::plan_parts::types::StepScope::CrossFile,
            operation: plans::Operation::Union {
                left: parts::Input::Step(parts::StepId(1)),
                right: parts::Input::Step(parts::StepId(0)),
            },
        },
    ];
    assert_eq!(
        super::service::frontier_of(&steps),
        vec![
            parts::Input::Step(parts::StepId(0)),
            parts::Input::Relation(Relation::SyntaxChild)
        ]
    );
}
fn projected_file_key_controls_scope() {
    let builder = lowering_builder();
    let input = parts::Input::Relation(Relation::Function);
    let scoped = super::service::scoped(&builder, input);
    let fields = scoped.fields.clone();
    for (column, expected) in [
        (2, ryo_values::domain::plan_parts::types::StepScope::PerFile),
        (
            1,
            ryo_values::domain::plan_parts::types::StepScope::CrossFile,
        ),
    ] {
        let operation = plans::Operation::Project {
            from: input,
            columns: vec![parts::ColumnIndex(column)],
            distinct: parts::RepeatedRows::Dropped,
        };
        assert_eq!(
            super::service::scope_of(
                &operation,
                std::slice::from_ref(&scoped),
                &[fields[column as usize].clone()]
            ),
            expected
        );
    }
}

/// A step that compares rows on a span, by kind.
#[derive(Clone, Copy, Debug)]
enum Compared {
    /// Join.
    Join,
    /// Antijoin.
    Antijoin,
    /// Ordered match.
    OrderedMatch,
    /// Group count.
    GroupCount,
    /// Group digest.
    GroupDigest,
    /// Closure.
    Closure,
    /// Ordered dfs.
    OrderedDfs,
    /// Cycles.
    Cycles,
    /// Union.
    Union,
    /// Intersection.
    Intersection,
    /// Difference.
    Difference,
    /// Distinct.
    Distinct,
}

/// Every kind of step that compares rows on a span.
const COMPARED: [Compared; 12] = [
    Compared::Join,
    Compared::Antijoin,
    Compared::OrderedMatch,
    Compared::GroupCount,
    Compared::GroupDigest,
    Compared::Closure,
    Compared::OrderedDfs,
    Compared::Cycles,
    Compared::Union,
    Compared::Intersection,
    Compared::Difference,
    Compared::Distinct,
];

/// A planned column of this kind.
fn column(kind: CellKind) -> PlannedColumn {
    PlannedColumn {
        name: ryo_values::domain::descriptions::types::ColumnName("c".into()),
        kind,
        unique: Uniqueness::Shared,
        words: ryo_relations::domain::terms::types::Vocabulary::Open,
    }
}

/// The step of this kind over `rows`, comparing them on the span at `span`; `other` is a column it reads besides.
fn compared(kind: Compared, rows: parts::Input, span: parts::ColumnIndex, other: parts::ColumnIndex) -> plans::Operation {
    match kind {
        Compared::Join => plans::Operation::Join {
            left: rows,
            right: rows,
            left_on: vec![span],
            right_on: vec![span],
            output: vec![],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        Compared::Antijoin => plans::Operation::Antijoin {
            left: rows,
            right: rows,
            left_on: vec![span],
            right_on: vec![span],
        },
        Compared::OrderedMatch => plans::Operation::OrderedMatch {
            left: rows,
            right: rows,
            left_on: vec![span],
            right_on: vec![span],
            left_order: other,
            right_order: other,
            output: vec![],
        },
        Compared::GroupCount => plans::Operation::GroupCount {
            from: rows,
            keys: vec![span],
            distinct: None,
        },
        Compared::GroupDigest => plans::Operation::GroupDigest {
            from: rows,
            keys: vec![span],
            over: other,
        },
        Compared::Closure => plans::Operation::Closure {
            edges: rows,
            keys: vec![],
            source: span,
            destination: span,
        },
        Compared::OrderedDfs => plans::Operation::OrderedDfs {
            from: rows,
            edges: rows,
            root: span,
            source: span,
            destination: span,
            order: other,
            edge: other,
            root_keys: vec![],
            edge_keys: vec![],
        },
        Compared::Cycles => plans::Operation::Cycles {
            edges: rows,
            source: span,
            destination: span,
        },
        Compared::Union => plans::Operation::Union {
            left: rows,
            right: rows,
        },
        Compared::Intersection => plans::Operation::Intersection {
            left: rows,
            right: rows,
        },
        Compared::Difference => plans::Operation::Difference {
            left: rows,
            right: rows,
        },
        Compared::Distinct => plans::Operation::Project {
            from: rows,
            columns: vec![span],
            distinct: parts::RepeatedRows::Dropped,
        },
    }
}

/// A step of this kind on spans alone, over the rows of a hosted relation made after the files (a span and a number, no file or
/// node), is refused: equal spans of two files would meet, merge or cancel.
fn refused_after_the_files(kind: Compared) {
    let mut builder = lowering_builder();
    super::service::push(
        &mut builder,
        None,
        plans::Operation::Hosted {
            relation: parts::HostedNumber(0),
        },
        vec![column(CellKind::Span), column(CellKind::Number)],
    );
    assert_eq!(builder.steps[0].scope, parts::StepScope::CrossFile);
    super::service::push(
        &mut builder,
        None,
        compared(kind, parts::Input::Step(parts::StepId(0)), parts::ColumnIndex(0), parts::ColumnIndex(1)),
        vec![column(CellKind::Span)],
    );
    assert!(
        matches!(
            super::service::refuse_spans_alone(&builder, 1),
            Err(LoweringError::CrossFileSpanKey { .. })
        ),
        "{kind:?} on spans alone after the files"
    );
}

/// Every kind of step on spans alone over per-file rows (the functions: id, name, file, span) is kept and runs in each file, where
/// its spans are places in that file.
fn kept_over_per_file_rows() {
    for kind in COMPARED {
        let mut builder = lowering_builder();
        super::service::push(
            &mut builder,
            None,
            compared(kind, parts::Input::Relation(Relation::Function), parts::ColumnIndex(3), parts::ColumnIndex(1)),
            vec![column(CellKind::Span)],
        );
        assert_eq!(super::service::refuse_spans_alone(&builder, 0), Ok(()), "{kind:?}");
        assert_eq!(builder.steps[0].scope, parts::StepScope::PerFile, "{kind:?}");
    }
}

fn join_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::Join);
}

fn antijoin_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::Antijoin);
}

fn ordered_match_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::OrderedMatch);
}

fn group_count_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::GroupCount);
}

fn group_digest_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::GroupDigest);
}

fn closure_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::Closure);
}

fn ordered_dfs_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::OrderedDfs);
}

fn cycles_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::Cycles);
}

fn union_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::Union);
}

fn intersection_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::Intersection);
}

fn difference_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::Difference);
}

fn distinct_on_spans_alone_after_the_files_is_refused() {
    refused_after_the_files(Compared::Distinct);
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("join_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || join_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("antijoin_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || antijoin_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordered_match_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || ordered_match_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("group_count_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || group_count_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("group_digest_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || group_digest_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("closure_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || closure_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordered_dfs_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || ordered_dfs_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("cycles_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || cycles_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("union_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || union_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("intersection_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || intersection_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("difference_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || difference_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("distinct_on_spans_alone_after_the_files_is_refused"),
        word: Word::Succeeds,
        input: || distinct_on_spans_alone_after_the_files_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_step_on_spans_alone_over_per_file_rows_is_kept_in_each_file"),
        word: Word::Succeeds,
        input: || kept_over_per_file_rows,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("frontier_keeps_only_file_to_project_dependencies"),
        word: Word::Succeeds,
        input: || frontier_keeps_only_file_to_project_dependencies,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("projected_file_key_controls_scope"),
        word: Word::Succeeds,
        input: || projected_file_key_controls_scope,
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
