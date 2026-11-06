//! A group of the contract's cases.
//!
//! The case `positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots`.

use super::support::ReaderOutcome;
use crate::cases::{Case, CaseName, Expect, Word};
use crate::fixtures::projects::notes_of;
use ryo::languages;
use ryo::languages::Project as _;

fn positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots() {
    use crate::domain::project_namespace::{
        constants::UNKNOWN_CONTEXT,
        service::memberships,
        types::{EntryKind, EntryShape, ImportedRoot, MembershipState, Presence},
    };
    use languages::ManifestItem as _;
    for (module, directory, incomplete, expected) in [
        (
            false,
            true,
            true,
            vec![EntryKind::Namespace, EntryKind::Unknown],
        ),
        (
            true,
            false,
            true,
            vec![EntryKind::Module, EntryKind::Unknown],
        ),
        (
            true,
            true,
            false,
            vec![EntryKind::Module, EntryKind::Namespace],
        ),
    ] {
        let roots = [ImportedRoot {
            file: languages::FileId(0),
            root: "acme".into(),
            path: "acme".into(),
        }];
        let search = [
            languages::FilePath("src".into()),
            languages::FilePath("lib".into()),
        ];
        let namespace_memberships =
            memberships(Some(&search), &roots, vec![], |candidate, shape| {
                let file = shape == EntryShape::File;
                if candidate.0.starts_with("lib/") && incomplete {
                    None
                } else {
                    Some(Presence::from(match candidate.0.as_ref() {
                        "src/acme.py" => module && file,
                        "src/acme" => directory && !file,
                        _ => false,
                    }))
                }
            });
        let derived = crate::domain::links::types::Linked {
            namespace_memberships,
            ..Default::default()
        };
        let request = languages::ProjectRequest {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            observed_entries: Default::default(),
            repository: languages::Repository::Given,
            root: languages::RootPath("".into()),
            files: vec![languages::FilePath("source.py".into())],
            manifests: vec![],
            notes: vec![notes_of("source.py", "import acme\n")],
            wanted: vec![languages::Relation::ProjectNamespaceMembership],
        };
        assert_eq!(
            derived
                .namespace_memberships
                .rows
                .iter()
                .map(|row| row.kind)
                .collect::<Vec<_>>(),
            expected
        );
        let positives = derived
            .namespace_memberships
            .rows
            .iter()
            .filter(|row| row.kind != EntryKind::Unknown)
            .collect::<Vec<_>>();
        assert!(!positives.is_empty());
        assert!(
            positives
                .iter()
                .all(|row| row.state == MembershipState::Present)
        );
        let mut output = ReaderOutcome::default();
        let mut symbols = languages::Symbols::default();
        let input = languages::ManifestInput {
            request: &request,
            derived: &derived,
        };
        (crate::domain::manifests::collections::manifests::project_namespace_membership::item::ProjectNamespaceMembershipItem::READER.read)(&input, &mut symbols, &mut output);
        assert_eq!(output.rows, expected.len());
        assert_eq!(
            output.partial,
            if incomplete {
                vec![UNKNOWN_CONTEXT.to_owned()]
            } else {
                vec![]
            }
        );
    }
}

/// The row of this case.
pub(super) const CASES: &[Case<(), (), ()>] = &[Case {
    name: CaseName("positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];

/// Runs the body of the case.
pub(super) fn subject((): ()) -> Result<(), ()> {
    positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots();
    Ok(())
}
