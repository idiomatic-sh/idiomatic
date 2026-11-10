//! A step on a span with a file key beside it, over rows made after the files, is kept: the file keeps two files apart.
use super::super::fixtures::{Compared, column, compared_with_file};
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

/// A step of this kind on a span with a file key beside it, over the rows of a hosted relation made after the files (a span, a
/// file and a number), is kept: the file keeps the rows of two files apart.
fn kept_with_a_file_key_after_the_files(kind: Compared) {
    let mut builder = lowering_builder();
    crate::domain::planning::service::push(
        &mut builder,
        None,
        plans::Operation::Hosted {
            relation: parts::HostedNumber(0),
        },
        vec![
            column(CellKind::Span),
            column(CellKind::File),
            column(CellKind::Number),
        ],
    );
    crate::domain::planning::service::push(
        &mut builder,
        None,
        compared_with_file(
            kind,
            parts::Input::Step(parts::StepId(0)),
            parts::ColumnIndex(0),
            parts::ColumnIndex(1),
            parts::ColumnIndex(2),
        ),
        vec![column(CellKind::Span), column(CellKind::File)],
    );
    assert_eq!(
        super::super::service::refuse_spans_alone(
            &builder,
            ryo_values::domain::plan_parts::types::StepId(1)
        ),
        Ok(()),
        "{kind:?} on a span and a file after the files"
    );
}

fn antijoin_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::Antijoin);
}

fn ordered_match_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::OrderedMatch);
}

fn group_count_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::GroupCount);
}

fn group_digest_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::GroupDigest);
}

fn closure_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::Closure);
}

fn ordered_dfs_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::OrderedDfs);
}

fn cycles_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::Cycles);
}

fn union_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::Union);
}

fn intersection_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::Intersection);
}

fn difference_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::Difference);
}

fn distinct_on_a_span_and_a_file_after_the_files_is_kept() {
    kept_with_a_file_key_after_the_files(Compared::Distinct);
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("antijoin_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || antijoin_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordered_match_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || ordered_match_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("group_count_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || group_count_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("group_digest_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || group_digest_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("closure_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || closure_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordered_dfs_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || ordered_dfs_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("cycles_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || cycles_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("union_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || union_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("intersection_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || intersection_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("difference_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || difference_on_a_span_and_a_file_after_the_files_is_kept,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("distinct_on_a_span_and_a_file_after_the_files_is_kept"),
        word: Word::Succeeds,
        input: || distinct_on_a_span_and_a_file_after_the_files_is_kept,
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
