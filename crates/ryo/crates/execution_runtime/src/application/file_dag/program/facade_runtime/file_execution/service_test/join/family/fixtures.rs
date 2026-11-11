//! The support of the `family` cases.
use crate::cases::{Case, CaseName, Expect, Word};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Departure, exercise,
};

pub(crate) type Input = DerivedKind;

pub(crate) type Value = DerivedKind;

pub(crate) type Failure = OperatingError;

pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("actual_Equality"),
        word: Word::Succeeds,
        input: || DerivedKind::Equality,
        expect: Expect::Value(DerivedKind::Equality),
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
];

pub(crate) fn subject(kind: DerivedKind) -> Result<DerivedKind, OperatingError> {
    exercise(kind, Departure::Succeeds)
}
