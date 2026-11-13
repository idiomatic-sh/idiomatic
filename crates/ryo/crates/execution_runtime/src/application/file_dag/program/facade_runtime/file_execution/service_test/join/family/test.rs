//! All nine real families reach the common admitted owner.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Departure,
    departed::{Departed, Refused, Seen},
    exercise,
};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;

/// The family an owner acquires.
type Input = DerivedKind;
/// The family the admitted owner answered for, with what its route saw, the lease its exit left
/// and the bytes still charged once the generation retired.
type Value = Departed;
type Failure = Refused;

/// A family read with nothing seen on the way, nothing left charged at the exit or after retirement.
const fn read(kind: DerivedKind) -> Departed {
    Departed {
        kind,
        along: Cow::Borrowed(&[]),
        leaked: None,
        retired: (0, 0, 0),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("actual_Equality"),
        word: Word::Succeeds,
        input: || DerivedKind::Equality,
        expect: Expect::Value(read(DerivedKind::Equality)),
    },
    Case {
        name: CaseName("actual_Adjacency"),
        word: Word::Succeeds,
        input: || DerivedKind::Adjacency,
        expect: Expect::Value(read(DerivedKind::Adjacency)),
    },
    Case {
        name: CaseName("actual_Membership"),
        word: Word::Succeeds,
        input: || DerivedKind::Membership,
        expect: Expect::Value(read(DerivedKind::Membership)),
    },
    Case {
        name: CaseName("actual_PartitionOrder"),
        word: Word::Succeeds,
        input: || DerivedKind::PartitionOrder,
        expect: Expect::Value(read(DerivedKind::PartitionOrder)),
    },
    Case {
        name: CaseName("actual_ClosureAdjacency"),
        word: Word::Succeeds,
        input: || DerivedKind::ClosureAdjacency,
        expect: Expect::Value(Departed {
            kind: DerivedKind::ClosureAdjacency,
            along: Cow::Borrowed(&[
                Seen::Advanced,
                Seen::Key(Cow::Borrowed(&[Cell::Number(Integer(0))])),
                Seen::Advanced,
                Seen::Key(Cow::Borrowed(&[Cell::Number(Integer(1))])),
                Seen::Exhausted,
            ]),
            leaked: None,
            retired: (0, 0, 0),
        }),
    },
    Case {
        name: CaseName("actual_CycleAdjacency"),
        word: Word::Succeeds,
        input: || DerivedKind::CycleAdjacency,
        expect: Expect::Value(read(DerivedKind::CycleAdjacency)),
    },
    Case {
        name: CaseName("actual_OrderedKeys"),
        word: Word::Succeeds,
        input: || DerivedKind::OrderedKeys,
        expect: Expect::Value(read(DerivedKind::OrderedKeys)),
    },
    Case {
        name: CaseName("actual_OrderedDfs"),
        word: Word::Succeeds,
        input: || DerivedKind::OrderedDfs,
        expect: Expect::Value(read(DerivedKind::OrderedDfs)),
    },
    Case {
        name: CaseName("actual_NativeLookup"),
        word: Word::Succeeds,
        input: || DerivedKind::NativeLookup,
        expect: Expect::Value(read(DerivedKind::NativeLookup)),
    },
];

/// Acquires the family's owner and lets it depart successfully.
fn subject(kind: Input) -> Result<Value, Failure> {
    exercise(kind, Departure::Succeeds)
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
