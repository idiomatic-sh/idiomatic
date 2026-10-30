//! The `validate` service: local free-function helpers, private to this use case: the problems a fixture is said to
//! have, the error a manifest failure is, the release and label of a definition a spec owns, the versions in use and
//! where a version is.

use std::collections::{BTreeMap, BTreeSet};

use super::errors::ValidateError;
use super::types::Version;
use crate::domain::definitions::types::ErrorFile;
use crate::domain::packages::service::{kind_directory, kind_file, kind_word};
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, Ownership, PackageName, PackageRoot, Release, ResolvedPackage,
};
use crate::domain::paths::types::RepoPath;
use crate::domain::proofs::types::{DefinitionLabel, Problem, ProofFile, ProofPath};
use crate::domain::scenario_proving::service::{release_directory, subject_of};
use crate::outbound::packages::types::PackageFile;

/// A problem of a fixture, said of `reason`.
pub(super) fn problem(reason: impl std::fmt::Display) -> Problem {
    Problem(reason.to_string().into())
}

/// The error a manifest failure of `file` is.
pub(super) fn manifest_error(
    file: ErrorFile<'_>,
) -> impl Fn(crate::outbound::manifest::errors::ManifestError) -> ValidateError + '_ {
    move |source| ValidateError::Manifest {
        file: file.0.to_owned(),
        source,
    }
}

/// What names an owned asset in a message: a rule or a template, its name, and the spec that owns it.
pub(super) fn owned_label(spec: &DefinitionLabel, file: &PackageFile) -> DefinitionLabel {
    let name = file.0.rsplit('/').nth(1).unwrap_or_default();
    let kind = if file.0.ends_with(kind_file(DefinitionKind::Rule).0) {
        kind_word(DefinitionKind::Rule).0
    } else {
        kind_word(DefinitionKind::Template).0
    };
    DefinitionLabel(format!("{kind} `{name}` of {}", spec.0))
}

/// The release of the rule or template a spec owns, whose file is `file` within the package: the spec's package and
/// version, the directory the file is in as its name.
pub(super) fn owned_release(spec: &Release, kind: DefinitionKind, file: &PackageFile) -> Release {
    let name = file.0.rsplit('/').nth(1).unwrap_or_default();
    Release {
        package: spec.package.clone(),
        kind,
        name: DefinitionName(name.into()),
        version: spec.version.clone(),
        ownership: Ownership::Owned(spec.name.clone()),
    }
}

/// A failure of checking a fixture's project, as a problem of the fixture.
pub(super) fn unchecked(reason: impl std::fmt::Display) -> Problem {
    problem(format_args!("the check fails: {reason}"))
}

/// A definition of a fixture's project that does not read, as a problem of the fixture.
pub(super) fn unread(
    file: &RepoPath,
) -> impl Fn(crate::outbound::manifest::errors::ManifestError) -> Problem + '_ {
    move |source| unchecked(format_args!("`{}`: {source}", file.0))
}

/// The versions in use of every definition the packages hold, with the paths of their definition files: a spec's
/// with its owned definitions'. A version in use is a definition's latest, or one the workspace's lock pins: a past
/// version nothing pins is not proved. The packages' layout is fixed, so their files, `listed`, say which versions
/// there are; no file is read: a version's files are read when it is proved.
pub(super) fn versions(
    packages: &BTreeMap<PackageName, ResolvedPackage>,
    listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
    pinned: &BTreeSet<Release>,
) -> Vec<Version> {
    use crate::domain::packages::service::{held_versions, latest, owned_assets};
    let _discovering = tracing::info_span!("versions").entered();
    let mut found = Vec::new();
    for (name, package) in packages {
        let files = listed
            .get(&package.directory)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let held = held_versions(files);
        for kind in [
            DefinitionKind::Rule,
            DefinitionKind::Spec,
            DefinitionKind::Template,
        ] {
            for (definition, offered) in held
                .range(
                    crate::domain::packages::types::DefinitionKey {
                        kind,
                        name: DefinitionName(Box::default()),
                    }..,
                )
                .take_while(|(each, _)| each.kind == kind)
            {
                let newest = latest(
                    &crate::domain::packages::types::VersionText(Box::default()),
                    offered.iter(),
                );
                for version in offered {
                    let release = Release {
                        package: name.clone(),
                        kind,
                        name: definition.name.clone(),
                        version: version.clone(),
                        ownership: Ownership::Shared,
                    };
                    if *version != newest && !pinned.contains(&release) {
                        continue;
                    }
                    let subject = subject_of(&release);
                    let directory_text = subject.directory.0.to_string();
                    let mut paths = vec![PackageFile(
                        format!("{directory_text}/{}", kind_file(kind).0).into(),
                    )];
                    if kind == DefinitionKind::Spec {
                        paths.extend(
                            owned_assets(
                                files,
                                &crate::domain::packages::types::ReleaseDirectory(
                                    directory_text.clone(),
                                ),
                            )
                            .into_iter()
                            .map(|owned| {
                                PackageFile(
                                    format!(
                                        "{directory_text}/{}/{}/{}",
                                        kind_directory(owned.kind).0,
                                        owned.name.0,
                                        kind_file(owned.kind).0
                                    )
                                    .into(),
                                )
                            }),
                        );
                    }
                    found.push(Version {
                        subject,
                        release,
                        package: package.directory.clone(),
                        files: paths,
                    });
                }
            }
        }
    }
    found
}

/// Where the version `release` is: its package's directory, among the workspace's packages, among those `located`
/// found, or in the name a package only a scenario's own `tastemaker.toml` names was given; and its directory there.
pub(super) fn place_of(
    release: &Release,
    packages: &BTreeMap<PackageName, ResolvedPackage>,
    located: &std::collections::HashMap<PackageName, PackageRoot>,
) -> Option<ProofFile> {
    let directory = packages
        .get(&release.package)
        .map(|package| package.directory.clone())
        .or_else(|| located.get(&release.package).cloned())
        .or_else(|| {
            release
                .package
                .0
                .split_once('@')
                .map(|(_, directory)| PackageRoot(directory.into()))
        })?;
    Some(ProofFile {
        directory,
        path: ProofPath(release_directory(release).0),
    })
}
