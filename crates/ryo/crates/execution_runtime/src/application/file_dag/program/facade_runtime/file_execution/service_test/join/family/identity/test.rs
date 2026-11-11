//! One cell per family and mismatch: the issued capability binds the owner generation, the
//! exact plan, the admitted population and the issued descriptor.
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Scenario, exercise,
};

type Input = (DerivedKind, Scenario);
type Value = DerivedKind;
type Failure = OperatingError;

/// One row per family and mismatch: the capability issued for one owner generation, plan,
/// population and descriptor is refused for any other.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_family_refuses_every_mismatched_capability"),
        word: Word::Refuses,
        input: || (DerivedKind::Equality, Scenario::Stale),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_equality_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::Equality, Scenario::OtherPlan),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_equality_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::Equality, Scenario::OtherPopulation),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_equality_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::Equality, Scenario::RetargetedDescriptor),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_adjacency_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::Adjacency, Scenario::Stale),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_adjacency_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::Adjacency, Scenario::OtherPlan),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_adjacency_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::Adjacency, Scenario::OtherPopulation),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_adjacency_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::Adjacency, Scenario::RetargetedDescriptor),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_membership_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::Membership, Scenario::Stale),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_membership_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::Membership, Scenario::OtherPlan),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_membership_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::Membership, Scenario::OtherPopulation),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_membership_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::Membership, Scenario::RetargetedDescriptor),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_partition_order_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::PartitionOrder, Scenario::Stale),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_partition_order_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::PartitionOrder, Scenario::OtherPlan),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_partition_order_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::PartitionOrder, Scenario::OtherPopulation),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_partition_order_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::PartitionOrder, Scenario::RetargetedDescriptor),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_closure_adjacency_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::ClosureAdjacency, Scenario::Stale),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_closure_adjacency_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::ClosureAdjacency, Scenario::OtherPlan),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_closure_adjacency_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::ClosureAdjacency, Scenario::OtherPopulation),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_closure_adjacency_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::ClosureAdjacency, Scenario::RetargetedDescriptor),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_cycle_adjacency_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::CycleAdjacency, Scenario::Stale),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_cycle_adjacency_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::CycleAdjacency, Scenario::OtherPlan),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_cycle_adjacency_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::CycleAdjacency, Scenario::OtherPopulation),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_cycle_adjacency_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::CycleAdjacency, Scenario::RetargetedDescriptor),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_ordered_keys_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedKeys, Scenario::Stale),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_ordered_keys_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedKeys, Scenario::OtherPlan),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_ordered_keys_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedKeys, Scenario::OtherPopulation),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_ordered_keys_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedKeys, Scenario::RetargetedDescriptor),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_ordered_dfs_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedDfs, Scenario::Stale),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_ordered_dfs_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedDfs, Scenario::OtherPlan),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_ordered_dfs_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedDfs, Scenario::OtherPopulation),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_ordered_dfs_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::OrderedDfs, Scenario::RetargetedDescriptor),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_native_lookup_family_refuses_a_stale_generation"),
        word: Word::Refuses,
        input: || (DerivedKind::NativeLookup, Scenario::Stale),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_native_lookup_family_refuses_another_plan"),
        word: Word::Refuses,
        input: || (DerivedKind::NativeLookup, Scenario::OtherPlan),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("the_native_lookup_family_refuses_another_population"),
        word: Word::Refuses,
        input: || (DerivedKind::NativeLookup, Scenario::OtherPopulation),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("the_native_lookup_family_refuses_a_retargeted_descriptor"),
        word: Word::Refuses,
        input: || (DerivedKind::NativeLookup, Scenario::RetargetedDescriptor),
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

/// Acquires the family's view with the mismatched capability.
fn subject((kind, scenario): Input) -> Result<Value, Failure> {
    exercise(kind, scenario)
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
