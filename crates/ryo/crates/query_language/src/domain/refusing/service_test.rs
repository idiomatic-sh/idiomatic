//! What `refusing` promises: a step on spans alone over rows made after the files is refused, and over per-file rows it is kept.
use super::fixtures::{COMPARED, Compared, column, compared};
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

/// A step of this kind on spans alone, over the rows of a hosted relation made after the files (a span and a number, no file or
/// node), is refused: equal spans of two files would meet, merge or cancel.
fn refused_after_the_files(kind: Compared) {
    let mut builder = lowering_builder();
    crate::domain::planning::service::push(
        &mut builder,
        None,
        plans::Operation::Hosted {
            relation: parts::HostedNumber(0),
        },
        vec![column(CellKind::Span), column(CellKind::Number)],
    );
    assert_eq!(builder.steps[0].scope, parts::StepScope::CrossFile);
    crate::domain::planning::service::push(
        &mut builder,
        None,
        compared(
            kind,
            parts::Input::Step(parts::StepId(0)),
            parts::ColumnIndex(0),
            parts::ColumnIndex(1),
        ),
        vec![column(CellKind::Span)],
    );
    assert!(
        matches!(
            super::service::refuse_spans_alone(
                &builder,
                ryo_values::domain::plan_parts::types::StepId(1)
            ),
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
        crate::domain::planning::service::push(
            &mut builder,
            None,
            compared(
                kind,
                parts::Input::Relation(Relation::Function),
                parts::ColumnIndex(3),
                parts::ColumnIndex(1),
            ),
            vec![column(CellKind::Span)],
        );
        assert_eq!(
            super::service::refuse_spans_alone(
                &builder,
                ryo_values::domain::plan_parts::types::StepId(0)
            ),
            Ok(()),
            "{kind:?}"
        );
        assert_eq!(
            builder.steps[0].scope,
            parts::StepScope::PerFile,
            "{kind:?}"
        );
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
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod file_key_test;

#[cfg(test)]
mod vertices_test;
