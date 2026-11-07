//! The test data of the `update` use case: the package `p` its rows move, held by the fake of the packages port, the
//! lock that pins it, the repository that holds both, and what an update leaves of a package in the repository.

use crate::inbound::use_cases::check::fixtures::{
    PACKAGE, RULE, SPEC, TEMPLATE, Workspace, repository_of,
};
use crate::outbound::packages::fixtures::fake::PackagesFake;
use crate::outbound::repository::fixtures::fake::RepositoryFake;

/// What goes wrong in a row, or where its package lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Nothing.
    None,
    /// A spec of the package names a dependency that is missing.
    MissingDependency,
    /// The lock cannot be written.
    LockWrite,
    /// The lock cannot be read.
    LockUnreadable,
    /// The package lies in the repository, at `p`, beside the workspace that names it.
    InRepository,
}

/// A lock that pins `marker` at its first version.
const LOCKED: &str = "version = 1\ndependencies = [\"p\"]\n\n[packages.p]\npath = \"p\"\n\n[packages.p.rules]\nmarker = \"0.1.0\"\n\n[packages.p.specs]\npart = \"0.1.0\"\n";

/// The files of the package `p`, by their path in it.
const PACKAGE_FILES: &[(&str, &str)] = &[
    ("package.toml", PACKAGE),
    ("rules/marker/0.1.0/rule.toml", RULE),
    ("rules/marker/0.2.0/rule.toml", RULE),
    ("specs/part/0.1.0/spec.toml", SPEC),
    ("specs/part/0.1.0/templates/errors/template.toml", TEMPLATE),
];

/// The package `p`, at `/p` or, when the row's problem puts it there, in the repository, with a third version of `marker` that holds no definition when the row moves to it, and a spec that
/// names a missing dependency when the row's problem is one.
#[must_use]
pub fn package_of(to: Option<&'static str>, problem: Problem) -> PackagesFake {
    let name = if problem == Problem::InRepository {
        "fake/p"
    } else {
        "p"
    };
    let mut package = PackagesFake::of(name, PACKAGE_FILES);
    if to == Some("0.3.0") {
        drop(package.files.insert(
            ("/p".into(), "rules/marker/0.3.0/README.md".into()),
            "No definition here.".into(),
        ));
    }
    if problem == Problem::MissingDependency {
        drop(
            package.files.insert(
                ("/p".into(), "specs/part/0.1.0/spec.toml".into()),
                SPEC.replacen(
                    "\n[[scenarios]]",
                    "\nrules=[{rule=\"absent/missing@0.1.0\",include=[\"**\"]}]\n\n[[scenarios]]",
                    1,
                )
                .into(),
            ),
        );
    }
    package
}

/// The repository of `workspace`, with the lock that pins `marker` at 0.1.0 and what the row's problem puts in it: the
/// package `p` beside the workspace, or a lock that cannot be read or written.
#[must_use]
pub fn repository_for(workspace: Workspace, problem: Problem) -> RepositoryFake {
    let mut repository = repository_of(workspace);
    // The lock pins `marker` at 0.1.0, which an update moves.
    drop(
        repository
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                crate::domain::paths::types::RepoPath("tastemaker.lock".into()),
                crate::domain::files::types::FileText(LOCKED.into()),
            ),
    );
    if problem == Problem::InRepository {
        let mut files = repository
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (path, text) in PACKAGE_FILES {
            drop(files.insert(
                crate::domain::paths::types::RepoPath(format!("p/{path}").into()),
                crate::domain::files::types::FileText((*text).into()),
            ));
        }
    }
    if problem == Problem::LockUnreadable {
        let _: bool = repository
            .read_failures
            .insert(crate::domain::paths::types::RepoPath(
                "tastemaker.lock".into(),
            ));
    }
    if problem == Problem::LockWrite {
        let _: bool = repository
            .write_failures
            .insert(crate::domain::paths::types::RepoPath(
                "tastemaker.lock".into(),
            ));
    }
    repository
}

/// The files of the package in the repository that `after` holds, joined, when they are not all the files it held: `update` moves
/// the lock and removes no version, so the one it moved from stays beside the one it moved to.
#[must_use]
pub fn removed(
    after: &std::collections::BTreeMap<
        crate::domain::paths::types::RepoPath,
        crate::domain::files::types::FileText,
    >,
) -> Option<String> {
    let left = after
        .keys()
        .filter(|path| path.0.starts_with("p/"))
        .map(|path| path.0.to_string())
        .collect::<Vec<_>>();
    let held = PACKAGE_FILES
        .iter()
        .map(|(path, _)| format!("p/{path}"))
        .collect::<Vec<_>>();
    (left != held).then(|| left.join(","))
}
