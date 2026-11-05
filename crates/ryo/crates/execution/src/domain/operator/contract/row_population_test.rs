//! The `row_population` service's cases, on values alone.
use super::service::semantics;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plans::types::OperationKind;
type Input = OperationKind;
type Value = u8;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("select_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Select,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("project_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Project,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("extend_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Extend,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("join_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Join,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("union_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Union,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("intersection_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Intersection,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("difference_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Difference,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("antijoin_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Antijoin,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("groupcount_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::GroupCount,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("groupdigest_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::GroupDigest,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("capture_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Capture,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("rank_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Rank,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("words_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Words,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("replace_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Replace,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("orderedmatch_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::OrderedMatch,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("axis_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Axis,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("closure_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Closure,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("cycles_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Cycles,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("ordereddfs_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::OrderedDfs,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("rows_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Rows,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("hosted_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::Hosted,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("spanbetween_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::SpanBetween,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("sourcematches_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::SourceMatches,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("rendersyntax_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::RenderSyntax,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("fitsrendered_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::FitsRendered,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("spanbounds_declares_its_driving_populations"),
        word: Word::Succeeds,
        input: || OperationKind::SpanBounds,
        expect: Expect::Value(1),
    },
];
fn subject(kind: Input) -> Result<Value, Failure> {
    Ok(semantics(kind).0.row_inputs.0)
}
#[test]
fn succeeds() {
    assert_eq!(
        CASES.len(),
        OperationKind::ALL.len(),
        "every registered kind has an independent expected row population"
    );
    run(CASES, Word::Succeeds, subject);
}
