//! A group of the contract's cases.
//!
//! The case `positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots`.

use super::support::ReaderOutcome;
use crate::fixtures::projects::notes_of;
use ryo::languages;

pub(crate) fn positive_namespace_facts_retain_actual_reader_partial_for_unobserved_roots() -> String
{
    let mut seen: Vec<String> = Vec::new();
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
        seen.push(format!(
            "{:?}",
            (derived
                .namespace_memberships
                .rows
                .iter()
                .map(|row| row.kind)
                .collect::<Vec<_>>())
                == (expected)
        ));
        let positives = derived
            .namespace_memberships
            .rows
            .iter()
            .filter(|row| row.kind != EntryKind::Unknown)
            .collect::<Vec<_>>();
        seen.push(format!("{:?}", !positives.is_empty()));
        seen.push(format!(
            "{:?}",
            positives
                .iter()
                .all(|row| row.state == MembershipState::Present)
        ));
        let mut output = ReaderOutcome::default();
        let mut symbols = languages::Symbols::default();
        let input = languages::ManifestInput {
            request: &request,
            derived: &derived,
        };
        (crate::domain::manifests::collections::manifests::project_namespace_membership::item::ProjectNamespaceMembershipItem::READER.read)(&input, &mut symbols, &mut output);
        seen.push(format!("{:?}", (output.rows) == (expected.len())));
        seen.push(format!(
            "{:?}",
            (output.partial)
                == (if incomplete {
                    vec![UNKNOWN_CONTEXT.to_owned()]
                } else {
                    vec![]
                })
        ));
    }
    seen.join("|")
}
