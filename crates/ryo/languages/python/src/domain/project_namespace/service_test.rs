//! Project namespace membership controls independent of successful dependency resolution.
use super::{candidates, memberships};
use crate::domain::project_namespace::types::{EntryKind, ImportedRoot, MembershipState};
use ryo::languages::{FileId, FilePath};

fn path(value: &str) -> FilePath {
    FilePath(value.into())
}
fn root(file: u32, name: &str) -> ImportedRoot {
    ImportedRoot {
        file: FileId(file),
        root: name.into(),
        path: name.into(),
    }
}

fn dotted(file: u32, name: &str) -> ImportedRoot {
    ImportedRoot {
        file: FileId(file),
        root: name.split('.').next().unwrap().into(),
        path: name.into(),
    }
}

#[test]
fn imported_root_membership_requires_complete_scope_or_positive_observation() {
    let imported = [root(0, "pkg"), root(1, "pkg")];
    let undeclared = memberships(None, &imported, Vec::new(), |_, _| None);
    assert_eq!(
        undeclared
            .rows
            .iter()
            .map(|row| row.state)
            .collect::<Vec<_>>(),
        vec![MembershipState::Unknown; 2]
    );
    let declared_empty = memberships(Some(&[]), &imported, Vec::new(), |_, _| None);
    assert_eq!(declared_empty.rows[0].state, MembershipState::Absent);
    let search = [path("src"), path("vendor")];
    let partly = memberships(Some(&search), &imported, Vec::new(), |candidate, _| {
        (candidate.0.as_ref() == "src/pkg.py").then_some(false)
    });
    assert_eq!(partly.rows[0].state, MembershipState::Unknown);
    let present = memberships(Some(&search), &imported, Vec::new(), |candidate, file| {
        (candidate.0.as_ref() == "src/pkg.py" && file).then_some(true)
    });
    assert_eq!(
        present.rows.iter().map(|row| row.state).collect::<Vec<_>>(),
        vec![MembershipState::Present; 2]
    );
}

#[test]
fn namespace_directory_and_missing_child_have_distinct_memberships() {
    let imported = [root(0, "pkg"), root(0, "other")];
    let search = [path("src")];
    assert_eq!(candidates(Some(&search), &imported).len(), 10);
    let rows = memberships(Some(&search), &imported, Vec::new(), |candidate, file| {
        Some(candidate.0.as_ref() == "src/pkg" && !file)
    })
    .rows;
    assert_eq!(
        rows.iter()
            .find(|row| row.root.as_ref() == "pkg")
            .map(|row| row.state),
        Some(MembershipState::Present)
    );
    assert_eq!(
        rows.iter()
            .find(|row| row.root.as_ref() == "other")
            .map(|row| row.state),
        Some(MembershipState::Absent)
    );
}

#[test]
fn namespace_root_stays_present_while_dotted_children_are_observed_separately() {
    let imported = [dotted(0, "acme.storage"), dotted(0, "acme.billing")];
    let search = [path("src")];
    let planned = candidates(Some(&search), &imported);
    assert!(planned.contains(&path("src/acme/storage")));
    assert!(planned.contains(&path("src/acme/billing/__init__.py")));
    let observed = |candidate: &FilePath, file: bool| {
        Some(match candidate.0.as_ref() {
            "src/acme" => !file,
            "src/acme/billing" => !file,
            "src/acme/billing/__init__.py" => file,
            _ => false,
        })
    };
    let rows = memberships(Some(&search), &imported, Vec::new(), observed).rows;
    let kind = |path: &str| {
        rows.iter()
            .find(|row| row.path.as_ref() == path)
            .map(|row| row.kind)
    };
    assert_eq!(kind("acme"), Some(EntryKind::Namespace));
    assert_eq!(kind("acme.storage"), Some(EntryKind::Absent));
    assert_eq!(kind("acme.billing"), Some(EntryKind::Package));
    assert_eq!(
        rows.iter()
            .find(|row| row.path.as_ref() == "acme")
            .map(|row| row.state),
        Some(MembershipState::Present)
    );
    let incomplete = memberships(Some(&search), &imported, Vec::new(), |candidate, file| {
        if candidate.0.as_ref() == "src/acme/storage" && !file {
            None
        } else {
            observed(candidate, file)
        }
    });
    assert_eq!(
        incomplete
            .rows
            .iter()
            .find(|row| row.path.as_ref() == "acme.storage")
            .map(|row| row.kind),
        Some(EntryKind::Unknown)
    );
}

#[test]
fn module_file_and_directory_collision_preserves_both_observed_prefixes() {
    let imported = [dotted(0, "acme.billing"), dotted(0, "acme.unknown")];
    let search = [path("src")];
    let observe = |candidate: &FilePath, file: bool| match (candidate.0.as_ref(), file) {
        ("src/acme.py", true) | ("src/acme/billing/__init__.py", true) => Some(true),
        ("src/acme", false) | ("src/acme/billing", false) => Some(true),
        ("src/acme/unknown", false) => None,
        _ => Some(false),
    };
    let rows = memberships(Some(&search), &imported, Vec::new(), observe).rows;
    let kind = |path: &str| {
        rows.iter()
            .find(|row| row.path.as_ref() == path)
            .map(|row| row.kind)
    };
    assert_eq!(kind("acme"), Some(EntryKind::Module));
    assert_eq!(kind("acme.billing"), Some(EntryKind::Package));
    assert_eq!(kind("acme.unknown"), Some(EntryKind::Unknown));
    assert_eq!(
        rows.iter()
            .filter(|row| row.path.as_ref() == "acme")
            .count(),
        2
    );
}

#[test]
fn dotted_child_add_delete_type_change_and_partial_observation_change_its_kind() {
    let imported = [dotted(0, "acme.billing")];
    let search = [path("src")];
    let state = |child_directory: Option<bool>, initializer: Option<bool>| {
        memberships(
            Some(&search),
            &imported,
            Vec::new(),
            |candidate, file| match (candidate.0.as_ref(), file) {
                ("src/acme", false) => Some(true),
                ("src/acme/billing", false) => child_directory,
                ("src/acme/billing/__init__.py", true) => initializer,
                _ => Some(false),
            },
        )
        .rows
        .into_iter()
        .find(|row| row.path.as_ref() == "acme.billing")
        .map(|row| row.kind)
        .expect("dotted observation")
    };
    assert_eq!(state(Some(false), Some(false)), EntryKind::Absent);
    assert_eq!(state(Some(true), Some(true)), EntryKind::Package);
    assert_eq!(state(Some(true), Some(false)), EntryKind::Namespace);
    assert_eq!(state(Some(false), Some(false)), EntryKind::Absent);
    assert_eq!(state(None, Some(false)), EntryKind::Unknown);
    assert_eq!(state(Some(true), None), EntryKind::Unknown);
}

#[test]
fn observed_package_in_another_declared_root_remains_a_real_project_entry() {
    let imported = [dotted(0, "acme.billing")];
    let observe = |candidate: &FilePath, file: bool| match (candidate.0.as_ref(), file) {
        ("a/acme", false) | ("a/acme/other", false) => Some(true),
        ("b/acme", false) | ("b/acme/billing", false) => Some(true),
        ("b/acme/__init__.py", true) | ("b/acme/billing/__init__.py", true) => Some(true),
        _ => Some(false),
    };
    for roots in [[path("a"), path("b")], [path("b"), path("a")]] {
        let rows = memberships(Some(&roots), &imported, Vec::new(), observe).rows;
        assert_eq!(
            rows.iter()
                .find(|row| row.path.as_ref() == "acme.billing")
                .map(|row| row.kind),
            Some(EntryKind::Package),
            "a real package in either declared root is retained as an observed fact"
        );
    }
}

#[test]
fn missing_or_conflicting_entry_cannot_prove_absence() {
    let imported = [root(0, "local")];
    let search = [path("")];
    let rows = memberships(Some(&search), &imported, Vec::new(), |candidate, _| {
        (candidate.0.as_ref() != "local.py").then_some(false)
    })
    .rows;
    assert_eq!(rows[0].state, MembershipState::Unknown);
}
