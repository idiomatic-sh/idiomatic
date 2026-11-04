//! Project namespace membership controls independent of successful dependency resolution.
use super::{candidates, memberships};
use crate::domain::project_namespace::types::{ImportedRoot, MembershipState};
use ryo::languages::{FileId, FilePath};

fn path(value: &str) -> FilePath {
    FilePath(value.into())
}
fn root(file: u32, name: &str) -> ImportedRoot {
    ImportedRoot {
        file: FileId(file),
        root: name.into(),
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
    assert_eq!(candidates(Some(&search), &imported).len(), 6);
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
fn missing_or_conflicting_entry_cannot_prove_absence() {
    let imported = [root(0, "local")];
    let search = [path("")];
    let rows = memberships(Some(&search), &imported, Vec::new(), |candidate, _| {
        (candidate.0.as_ref() != "local.py").then_some(false)
    })
    .rows;
    assert_eq!(rows[0].state, MembershipState::Unknown);
}
