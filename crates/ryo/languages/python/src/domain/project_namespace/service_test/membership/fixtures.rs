//! The support of the group's tests.
use super::super::super::service::{candidates, memberships};
use crate::domain::project_namespace::types::EntryKind;
use crate::domain::project_namespace::types::{EntryShape, ImportedRoot, Presence};
use ryo::languages::{FileId, FilePath};

/// Adapt an observation written over booleans (is it a file, does it exist) to the typed callback.
pub(super) fn typed(
    observe: impl Fn(&FilePath, bool) -> Option<bool>,
) -> impl Fn(&FilePath, EntryShape) -> Option<Presence> {
    move |candidate, shape| observe(candidate, shape == EntryShape::File).map(Presence::from)
}

pub(super) fn path(value: &str) -> FilePath {
    FilePath(value.into())
}

pub(super) fn root(file: u32, name: &str) -> ImportedRoot {
    ImportedRoot {
        file: FileId(file),
        root: name.into(),
        path: name.into(),
    }
}

pub(super) fn dotted(file: u32, name: &str) -> ImportedRoot {
    ImportedRoot {
        file: FileId(file),
        root: name.split('.').next().unwrap().into(),
        path: name.into(),
    }
}

pub(super) fn imported_root_membership_requires_complete_scope_or_positive_observation() -> String {
    let mut checks: Vec<String> = Vec::new();
    let imported = [root(0, "pkg"), root(1, "pkg")];
    let undeclared = memberships(None, &imported, Vec::new(), typed(|_, _| None));
    checks.push(format!(
        "{:?}",
        undeclared
            .rows
            .iter()
            .map(|row| row.state)
            .collect::<Vec<_>>()
    ));
    let declared_empty = memberships(Some(&[]), &imported, Vec::new(), typed(|_, _| None));
    checks.push(format!("{:?}", declared_empty.rows[0].state));
    let search = [path("src"), path("vendor")];
    let partly = memberships(
        Some(&search),
        &imported,
        Vec::new(),
        typed(|candidate, _| (candidate.0.as_ref() == "src/pkg.py").then_some(false)),
    );
    checks.push(format!("{:?}", partly.rows[0].state));
    let present = memberships(
        Some(&search),
        &imported,
        Vec::new(),
        typed(|candidate, file| (candidate.0.as_ref() == "src/pkg.py" && file).then_some(true)),
    );
    checks.push(format!(
        "{:?}",
        present.rows.iter().map(|row| row.state).collect::<Vec<_>>()
    ));
    checks.join("|")
}

pub(super) fn namespace_directory_and_missing_child_have_distinct_memberships() -> String {
    let mut checks: Vec<String> = Vec::new();
    let imported = [root(0, "pkg"), root(0, "other")];
    let search = [path("src")];
    checks.push(format!("{:?}", candidates(Some(&search), &imported).len()));
    let rows = memberships(
        Some(&search),
        &imported,
        Vec::new(),
        typed(|candidate, file| Some(candidate.0.as_ref() == "src/pkg" && !file)),
    )
    .rows;
    checks.push(format!(
        "{:?}",
        rows.iter()
            .find(|row| row.root.as_ref() == "pkg")
            .map(|row| row.state)
    ));
    checks.push(format!(
        "{:?}",
        rows.iter()
            .find(|row| row.root.as_ref() == "other")
            .map(|row| row.state)
    ));
    checks.join("|")
}

pub(super) fn namespace_root_stays_present_while_dotted_children_are_observed_separately() -> String
{
    let mut checks: Vec<String> = Vec::new();
    let imported = [dotted(0, "acme.storage"), dotted(0, "acme.billing")];
    let search = [path("src")];
    let planned = candidates(Some(&search), &imported);
    checks.push(format!("{}", planned.contains(&path("src/acme/storage"))));
    checks.push(format!(
        "{}",
        planned.contains(&path("src/acme/billing/__init__.py"))
    ));
    let observed = |candidate: &FilePath, file: bool| {
        Some(match candidate.0.as_ref() {
            "src/acme" => !file,
            "src/acme/billing" => !file,
            "src/acme/billing/__init__.py" => file,
            _ => false,
        })
    };
    let rows = memberships(Some(&search), &imported, Vec::new(), typed(observed)).rows;
    let kind = |path: &str| {
        rows.iter()
            .find(|row| row.path.as_ref() == path)
            .map(|row| row.kind)
    };
    checks.push(format!("{:?}", kind("acme")));
    checks.push(format!("{:?}", kind("acme.storage")));
    checks.push(format!("{:?}", kind("acme.billing")));
    checks.push(format!(
        "{:?}",
        rows.iter()
            .find(|row| row.path.as_ref() == "acme")
            .map(|row| row.state)
    ));
    let incomplete = memberships(
        Some(&search),
        &imported,
        Vec::new(),
        typed(|candidate, file| {
            if candidate.0.as_ref() == "src/acme/storage" && !file {
                None
            } else {
                observed(candidate, file)
            }
        }),
    );
    checks.push(format!(
        "{:?}",
        incomplete
            .rows
            .iter()
            .find(|row| row.path.as_ref() == "acme.storage")
            .map(|row| row.kind)
    ));
    checks.join("|")
}

pub(super) fn module_file_and_directory_collision_preserves_both_observed_prefixes() -> String {
    let mut checks: Vec<String> = Vec::new();
    let imported = [dotted(0, "acme.billing"), dotted(0, "acme.unknown")];
    let search = [path("src")];
    let observe = |candidate: &FilePath, file: bool| match (candidate.0.as_ref(), file) {
        ("src/acme.py", true) | ("src/acme/billing/__init__.py", true) => Some(true),
        ("src/acme", false) | ("src/acme/billing", false) => Some(true),
        ("src/acme/unknown", false) => None,
        _ => Some(false),
    };
    let rows = memberships(Some(&search), &imported, Vec::new(), typed(observe)).rows;
    let kind = |path: &str| {
        rows.iter()
            .find(|row| row.path.as_ref() == path)
            .map(|row| row.kind)
    };
    checks.push(format!("{:?}", kind("acme")));
    checks.push(format!("{:?}", kind("acme.billing")));
    checks.push(format!("{:?}", kind("acme.unknown")));
    checks.push(format!(
        "{:?}",
        rows.iter()
            .filter(|row| row.path.as_ref() == "acme")
            .map(|row| (row.kind, row.state))
            .collect::<Vec<_>>()
    ));
    checks.push(format!(
        "{:?}",
        rows.iter()
            .filter(|row| row.path.as_ref() == "acme")
            .count()
    ));
    checks.join("|")
}

pub(super) fn dotted_child_add_delete_type_change_and_partial_observation_change_its_kind() -> String
{
    let mut checks: Vec<String> = Vec::new();
    let imported = [dotted(0, "acme.billing")];
    let search = [path("src")];
    let state = |child_directory: Option<bool>, initializer: Option<bool>| {
        memberships(
            Some(&search),
            &imported,
            Vec::new(),
            typed(|candidate, file| match (candidate.0.as_ref(), file) {
                ("src/acme", false) => Some(true),
                ("src/acme/billing", false) => child_directory,
                ("src/acme/billing/__init__.py", true) => initializer,
                _ => Some(false),
            }),
        )
        .rows
        .into_iter()
        .find(|row| row.path.as_ref() == "acme.billing")
        .map(|row| row.kind)
        .expect("dotted observation")
    };
    checks.push(format!("{:?}", state(Some(false), Some(false))));
    checks.push(format!("{:?}", state(Some(true), Some(true))));
    checks.push(format!("{:?}", state(Some(true), Some(false))));
    checks.push(format!("{:?}", state(Some(false), Some(false))));
    checks.push(format!("{:?}", state(None, Some(false))));
    checks.push(format!("{:?}", state(Some(true), None)));
    checks.join("|")
}

pub(super) fn observed_package_in_another_declared_root_remains_a_real_project_entry() -> String {
    let mut checks: Vec<String> = Vec::new();
    let imported = [dotted(0, "acme.billing")];
    let observe = |candidate: &FilePath, file: bool| match (candidate.0.as_ref(), file) {
        ("a/acme", false) | ("a/acme/other", false) => Some(true),
        ("b/acme", false) | ("b/acme/billing", false) => Some(true),
        ("b/acme/__init__.py", true) | ("b/acme/billing/__init__.py", true) => Some(true),
        _ => Some(false),
    };
    for roots in [[path("a"), path("b")], [path("b"), path("a")]] {
        let rows = memberships(Some(&roots), &imported, Vec::new(), typed(observe)).rows;
        checks.push(format!(
            "{:?}",
            rows.iter()
                .find(|row| row.path.as_ref() == "acme.billing")
                .map(|row| row.kind)
        ));
    }
    checks.join("|")
}

pub(super) fn missing_or_conflicting_entry_cannot_prove_absence() -> String {
    let mut checks: Vec<String> = Vec::new();
    let imported = [root(0, "local")];
    let search = [path("")];
    let rows = memberships(
        Some(&search),
        &imported,
        Vec::new(),
        typed(|candidate, _| (candidate.0.as_ref() != "local.py").then_some(false)),
    )
    .rows;
    checks.push(format!("{:?}", rows[0].state));
    checks.join("|")
}

pub(super) fn positive_entries_keep_uncertainty_from_an_unobserved_search_root() -> String {
    let mut checks: Vec<String> = Vec::new();
    let imported = [root(0, "acme")];
    let search = [path("src"), path("lib")];
    for (module, expected) in [(false, EntryKind::Namespace), (true, EntryKind::Module)] {
        let rows = memberships(
            Some(&search),
            &imported,
            Vec::new(),
            typed(|candidate, file| {
                if candidate.0.starts_with("lib/") {
                    None
                } else {
                    Some(match candidate.0.as_ref() {
                        "src/acme.py" => module && file,
                        "src/acme" => !module && !file,
                        _ => false,
                    })
                }
            }),
        )
        .rows;
        checks.push(format!(
            "{:?}",
            rows.iter()
                .map(|row| (row.kind, row.state))
                .collect::<Vec<_>>()
        ));
        checks.push(format!(
            "{}",
            rows.iter()
                .all(|row| row.file == FileId(0) && &*row.root == "acme" && &*row.path == "acme")
        ));
    }
    checks.join("|")
}
