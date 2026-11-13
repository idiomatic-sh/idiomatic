//! Project namespace membership controls independent of successful dependency resolution.

use super::fixtures::{
    dotted_child_add_delete_type_change_and_partial_observation_change_its_kind,
    imported_root_membership_requires_complete_scope_or_positive_observation,
    missing_or_conflicting_entry_cannot_prove_absence,
    module_file_and_directory_collision_preserves_both_observed_prefixes,
    namespace_directory_and_missing_child_have_distinct_memberships,
    namespace_root_stays_present_while_dotted_children_are_observed_separately,
    observed_package_in_another_declared_root_remains_a_real_project_entry,
    positive_entries_keep_uncertainty_from_an_unobserved_search_root,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `imported_root_membership_requires_complete_scope_or_positive_observation` scenario.
    ImportedRootMembershipRequiresCompleteScopeOrPositiveObservation,
    /// The `namespace_directory_and_missing_child_have_distinct_memberships` scenario.
    NamespaceDirectoryAndMissingChildHaveDistinctMemberships,
    /// The `namespace_root_stays_present_while_dotted_children_are_observed_separately` scenario.
    NamespaceRootStaysPresentWhileDottedChildrenAreObservedSeparately,
    /// The `module_file_and_directory_collision_preserves_both_observed_prefixes` scenario.
    ModuleFileAndDirectoryCollisionPreservesBothObservedPrefixes,
    /// The `dotted_child_add_delete_type_change_and_partial_observation_change_its_kind` scenario.
    DottedChildAddDeleteTypeChangeAndPartialObservationChangeItsKind,
    /// The `observed_package_in_another_declared_root_remains_a_real_project_entry` scenario.
    ObservedPackageInAnotherDeclaredRootRemainsARealProjectEntry,
    /// The `missing_or_conflicting_entry_cannot_prove_absence` scenario.
    MissingOrConflictingEntryCannotProveAbsence,
    /// The `positive_entries_keep_uncertainty_from_an_unobserved_search_root` scenario.
    PositiveEntriesKeepUncertaintyFromAnUnobservedSearchRoot,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("imported_root_membership_requires_complete_scope_or_positive_observation"),
        word: Word::Succeeds,
        input: || Given::ImportedRootMembershipRequiresCompleteScopeOrPositiveObservation,
        expect: Expect::Value(
            "[Unknown, Unknown]|Absent|Unknown|[Present, Unknown, Present, Unknown]",
        ),
    },
    Case {
        name: CaseName("namespace_directory_and_missing_child_have_distinct_memberships"),
        word: Word::Succeeds,
        input: || Given::NamespaceDirectoryAndMissingChildHaveDistinctMemberships,
        expect: Expect::Value("10|Some(Present)|Some(Absent)"),
    },
    Case {
        name: CaseName(
            "namespace_root_stays_present_while_dotted_children_are_observed_separately",
        ),
        word: Word::Succeeds,
        input: || Given::NamespaceRootStaysPresentWhileDottedChildrenAreObservedSeparately,
        expect: Expect::Value(
            "true|true|Some(Namespace)|Some(Absent)|Some(Package)|Some(Present)|Some(Unknown)",
        ),
    },
    Case {
        name: CaseName("module_file_and_directory_collision_preserves_both_observed_prefixes"),
        word: Word::Succeeds,
        input: || Given::ModuleFileAndDirectoryCollisionPreservesBothObservedPrefixes,
        expect: Expect::Value(
            "Some(Module)|Some(Package)|Some(Unknown)|[(Module, Present), (Namespace, Present)]|2",
        ),
    },
    Case {
        name: CaseName(
            "dotted_child_add_delete_type_change_and_partial_observation_change_its_kind",
        ),
        word: Word::Succeeds,
        input: || Given::DottedChildAddDeleteTypeChangeAndPartialObservationChangeItsKind,
        expect: Expect::Value(
            "Some(Absent)|Some(Package)|Some(Namespace)|Some(Absent)|Some(Unknown)|Some(Unknown)",
        ),
    },
    Case {
        name: CaseName("observed_package_in_another_declared_root_remains_a_real_project_entry"),
        word: Word::Succeeds,
        input: || Given::ObservedPackageInAnotherDeclaredRootRemainsARealProjectEntry,
        expect: Expect::Value("Some(Package)|Some(Package)"),
    },
    Case {
        name: CaseName("missing_or_conflicting_entry_cannot_prove_absence"),
        word: Word::Succeeds,
        input: || Given::MissingOrConflictingEntryCannotProveAbsence,
        expect: Expect::Value("Unknown"),
    },
    Case {
        name: CaseName("positive_entries_keep_uncertainty_from_an_unobserved_search_root"),
        word: Word::Succeeds,
        input: || Given::PositiveEntriesKeepUncertaintyFromAnUnobservedSearchRoot,
        expect: Expect::Value(
            "[(Namespace, Present), (Unknown, Unknown)]|true|[(Module, Present), (Unknown, Unknown)]|true",
        ),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ImportedRootMembershipRequiresCompleteScopeOrPositiveObservation => {
            imported_root_membership_requires_complete_scope_or_positive_observation()
        }
        Given::NamespaceDirectoryAndMissingChildHaveDistinctMemberships => {
            namespace_directory_and_missing_child_have_distinct_memberships()
        }
        Given::NamespaceRootStaysPresentWhileDottedChildrenAreObservedSeparately => {
            namespace_root_stays_present_while_dotted_children_are_observed_separately()
        }
        Given::ModuleFileAndDirectoryCollisionPreservesBothObservedPrefixes => {
            module_file_and_directory_collision_preserves_both_observed_prefixes()
        }
        Given::DottedChildAddDeleteTypeChangeAndPartialObservationChangeItsKind => {
            dotted_child_add_delete_type_change_and_partial_observation_change_its_kind()
        }
        Given::ObservedPackageInAnotherDeclaredRootRemainsARealProjectEntry => {
            observed_package_in_another_declared_root_remains_a_real_project_entry()
        }
        Given::MissingOrConflictingEntryCannotProveAbsence => {
            missing_or_conflicting_entry_cannot_prove_absence()
        }
        Given::PositiveEntriesKeepUncertaintyFromAnUnobservedSearchRoot => {
            positive_entries_keep_uncertainty_from_an_unobserved_search_root()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
