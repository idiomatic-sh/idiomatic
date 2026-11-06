//! All thirteen real families reach the common admitted owner or actual native invocation.
use super::super::fixtures::{RawCase, observe};
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::exercise;
use crate::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::operating::errors::OperatingError,
};
use ryo_derived::domain::contract::types::DerivedKind;
const CASES: &[Case<DerivedKind, DerivedKind, OperatingError>] = &[
    Case {
        name: CaseName("actual_Equality"),
        word: Word::Succeeds,
        input: || DerivedKind::Equality,
        expect: Expect::Value(DerivedKind::Equality),
    },
    Case {
        name: CaseName("actual_PredicateMatches"),
        word: Word::Succeeds,
        input: || DerivedKind::PredicateMatches,
        expect: Expect::Value(DerivedKind::PredicateMatches),
    },
    Case {
        name: CaseName("actual_CandidatePostings"),
        word: Word::Succeeds,
        input: || DerivedKind::CandidatePostings,
        expect: Expect::Value(DerivedKind::CandidatePostings),
    },
    Case {
        name: CaseName("actual_DistinctRepresentatives"),
        word: Word::Succeeds,
        input: || DerivedKind::DistinctRepresentatives,
        expect: Expect::Value(DerivedKind::DistinctRepresentatives),
    },
    Case {
        name: CaseName("actual_Adjacency"),
        word: Word::Succeeds,
        input: || DerivedKind::Adjacency,
        expect: Expect::Value(DerivedKind::Adjacency),
    },
    Case {
        name: CaseName("actual_Membership"),
        word: Word::Succeeds,
        input: || DerivedKind::Membership,
        expect: Expect::Value(DerivedKind::Membership),
    },
    Case {
        name: CaseName("actual_PartitionOrder"),
        word: Word::Succeeds,
        input: || DerivedKind::PartitionOrder,
        expect: Expect::Value(DerivedKind::PartitionOrder),
    },
    Case {
        name: CaseName("actual_ClosureAdjacency"),
        word: Word::Succeeds,
        input: || DerivedKind::ClosureAdjacency,
        expect: Expect::Value(DerivedKind::ClosureAdjacency),
    },
    Case {
        name: CaseName("actual_CycleAdjacency"),
        word: Word::Succeeds,
        input: || DerivedKind::CycleAdjacency,
        expect: Expect::Value(DerivedKind::CycleAdjacency),
    },
    Case {
        name: CaseName("actual_OrderedKeys"),
        word: Word::Succeeds,
        input: || DerivedKind::OrderedKeys,
        expect: Expect::Value(DerivedKind::OrderedKeys),
    },
    Case {
        name: CaseName("actual_OrderedDfs"),
        word: Word::Succeeds,
        input: || DerivedKind::OrderedDfs,
        expect: Expect::Value(DerivedKind::OrderedDfs),
    },
    Case {
        name: CaseName("actual_NativeLookup"),
        word: Word::Succeeds,
        input: || DerivedKind::NativeLookup,
        expect: Expect::Value(DerivedKind::NativeLookup),
    },
    Case {
        name: CaseName("actual_RawKeyOrder"),
        word: Word::Succeeds,
        input: || DerivedKind::RawKeyOrder,
        expect: Expect::Value(DerivedKind::RawKeyOrder),
    },
];
fn subject(kind: DerivedKind) -> Result<DerivedKind, OperatingError> {
    if kind == DerivedKind::RawKeyOrder {
        observe(RawCase::Complete)?;
        return Ok(DerivedKind::RawKeyOrder);
    }
    exercise(kind)
}
#[test]
fn succeeds() {
    let mut declared = CASES.iter().map(|case| (case.input)()).collect::<Vec<_>>();
    declared.sort_by_key(|kind| kind.ordinal());
    assert_eq!(
        declared,
        DerivedKind::ALL,
        "the independent case table must contain every authoritative family exactly once"
    );
    run(CASES, Word::Succeeds, subject);
}
