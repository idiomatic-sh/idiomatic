//! One cell per family and mismatch: the issued capability binds the owner generation, the
//! exact plan, the admitted population and the issued descriptor.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Departure,
    departed::{Departed, Refused},
    exercise,
};
use std::borrow::Cow;

type Input = (DerivedKind, Departure);
/// The family read, or the refusal, with what the route saw, the lease its exit left and the bytes
/// still charged once the generation retired.
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

/// A refusal with nothing seen on the way, nothing left charged at the exit or after retirement.
const fn refused(error: OperatingError) -> Refused {
    Refused {
        error,
        along: Cow::Borrowed(&[]),
        leaked: None,
        retired: (0, 0, 0),
    }
}

/// One row per family and mismatch: the capability issued for one owner generation, plan,
/// population and descriptor is refused for any other.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "the_issued_capability_reads_the_view_of_its_own_generation_plan_and_population",
        ),
        word: Word::Succeeds,
        input: || (DerivedKind::Equality, Departure::Succeeds),
        expect: Expect::Value(read(DerivedKind::Equality)),
    },
    Case {
        name: CaseName("every_family_refuses_every_mismatched_capability"),
        word: Word::Refuses,
        input: || (DerivedKind::Equality, Departure::Stale),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_equality_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::Equality, Departure::OtherPlan),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_equality_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::Equality, Departure::OtherPopulation),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_equality_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::Equality, Departure::RetargetedDescriptor),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_adjacency_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::Adjacency, Departure::Stale),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_adjacency_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::Adjacency, Departure::OtherPlan),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_adjacency_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::Adjacency, Departure::OtherPopulation),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_adjacency_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::Adjacency, Departure::RetargetedDescriptor),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_membership_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::Membership, Departure::Stale),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_membership_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::Membership, Departure::OtherPlan),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_membership_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::Membership, Departure::OtherPopulation),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_membership_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::Membership, Departure::RetargetedDescriptor),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_partition_order_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::PartitionOrder, Departure::Stale),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_partition_order_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::PartitionOrder, Departure::OtherPlan),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_partition_order_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::PartitionOrder, Departure::OtherPopulation),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_partition_order_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::PartitionOrder, Departure::RetargetedDescriptor),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_closure_adjacency_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::ClosureAdjacency, Departure::Stale),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_closure_adjacency_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::ClosureAdjacency, Departure::OtherPlan),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_closure_adjacency_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::ClosureAdjacency, Departure::OtherPopulation),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_closure_adjacency_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || {
            (
                DerivedKind::ClosureAdjacency,
                Departure::RetargetedDescriptor,
            )
        },
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_cycle_adjacency_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::CycleAdjacency, Departure::Stale),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_cycle_adjacency_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::CycleAdjacency, Departure::OtherPlan),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_cycle_adjacency_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::CycleAdjacency, Departure::OtherPopulation),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_cycle_adjacency_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::CycleAdjacency, Departure::RetargetedDescriptor),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_ordered_keys_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedKeys, Departure::Stale),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_ordered_keys_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedKeys, Departure::OtherPlan),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_ordered_keys_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedKeys, Departure::OtherPopulation),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_ordered_keys_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedKeys, Departure::RetargetedDescriptor),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_ordered_dfs_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedDfs, Departure::Stale),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_ordered_dfs_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedDfs, Departure::OtherPlan),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_ordered_dfs_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedDfs, Departure::OtherPopulation),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_ordered_dfs_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedDfs, Departure::RetargetedDescriptor),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_native_lookup_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::NativeLookup, Departure::Stale),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_native_lookup_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::NativeLookup, Departure::OtherPlan),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
    Case {
        name: CaseName("the_native_lookup_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::NativeLookup, Departure::OtherPopulation),
        expect: Expect::Error(refused(OperatingError::NotRead)),
    },
    Case {
        name: CaseName("the_native_lookup_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::NativeLookup, Departure::RetargetedDescriptor),
        expect: Expect::Error(refused(OperatingError::WrongOperation)),
    },
];

/// Acquires the family's view with the mismatched capability.
fn subject((kind, departure): Input) -> Result<Value, Failure> {
    exercise(kind, departure)
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
