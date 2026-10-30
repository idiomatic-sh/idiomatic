//! The `dependencies` service: free functions named by verb that find the requirements of the packages of a repository, with the
//! package of it each needs when it holds one, with no IO and no ports.

use ryo::languages;

use crate::domain::packages::constants::{
    ALTERNATIVES, ARBITRARY_EQUAL, AUTHORITY, CLAUSES, DIRECT, EQUAL, GIT_SCHEME, MARKER, WILDCARD,
};
use crate::domain::packages::types::Specifier;
use crate::domain::patterns::constants::SEPARATOR;
use crate::domain::patterns::types::Part;

use crate::domain::packages::service::{
    causes_of, directory_of, file_of, members_of, normalized, parsed,
};
use crate::domain::packages::types::{
    Checkout, Declared, Dependency, Location, Marker, Membership, Naming, Normalized, Package,
    Packages, Parsed, Redirect, Reference, Replacement, Requirement, Revision, Target, Wired,
    Wiring, Written,
};

/// The name a package is known by, normalized; none for a manifest that declares no package.
fn name_of(declared: &Declared) -> Option<Normalized> {
    match &declared.name {
        Naming::Named(name) => Some(normalized(name)),
        Naming::Unnamed => None,
    }
}

/// Where a requirement sends its package, other than the package that makes it: the package in the directory it names, or else the one
/// of its name, which the manifest may send elsewhere for each condition; no package where the repository holds none. A name sent
/// outside the repository has none, even when a package of the repository has that name; one sent to the workspace is found by name.
fn targets_of<'a>(
    packages: &'a Packages,
    package: &'a Package,
    requirement: &Requirement,
    read: &Parsed,
) -> Vec<Target<'a>> {
    let in_directory = |location: &Location| {
        packages.packages.iter().find(|other| {
            other.manifest != package.manifest && directory_of(&other.manifest).0 == &*location.0
        })
    };
    let by_name = || {
        packages.packages.iter().find(|other| {
            other.manifest != package.manifest
                && name_of(&other.declared).is_some_and(|name| name == read.name)
        })
    };
    let own = |found: Option<&'a Package>| Target {
        package: found,
        condition: None,
        origin: requirement.origin,
        checkout: None,
    };
    match &requirement.reference {
        Reference::Unresolvable => vec![own(None)],
        Reference::Located(location) => vec![own(in_directory(location))],
        Reference::Named => {
            let sent: Vec<&Redirect> = package
                .declared
                .sources
                .iter()
                .filter(|redirect| normalized(&redirect.name) == read.name)
                .collect();
            if sent.is_empty() {
                return vec![own(by_name())];
            }
            sent.into_iter()
                .map(|redirect| Target {
                    package: match (&redirect.location, redirect.origin) {
                        (_, languages::DependencyOrigin::Workspace) => by_name(),
                        (Some(location), _) => in_directory(location),
                        (None, _) => None,
                    },
                    condition: Some(&redirect.marker),
                    origin: redirect.origin,
                    checkout: Some(&redirect.checkout),
                })
                .collect()
        }
    }
}

/// The condition of a requirement whose source carries one too: both must hold.
fn conditioned(requirement: &Marker, source: Option<&Marker>) -> Marker {
    match source {
        Some(source) if !source.0.is_empty() && requirement.0.is_empty() => source.clone(),
        Some(source) if !source.0.is_empty() => {
            Marker(format!("({}) and ({})", requirement.0, source.0).into())
        }
        Some(_) | None => requirement.clone(),
    }
}

/// The revision a requirement names, or else the one its source names: a requirement's own wins.
fn checkout_with(requirement: &Checkout, source: Option<&Checkout>) -> Checkout {
    match source {
        Some(source) if *requirement == Checkout::default() => source.clone(),
        Some(_) | None => requirement.clone(),
    }
}

/// What the manifests put in the place of a requirement of `package` on `name`: the first of the requirements of the same name that the
/// root of its install lists in uv's `override-dependencies` — a workspace that holds it, or its own manifest when no workspace holds
/// it; empty when none does. uv reads the overrides of the workspace root alone: a member's own are not applied.
fn replacement_of(
    packages: &Packages,
    members: &[Membership],
    package: &Package,
    name: &Normalized,
) -> Replacement {
    let member = members.iter().any(|membership| {
        membership.package == package.manifest && membership.workspace != package.manifest
    });
    packages
        .packages
        .iter()
        .filter(|each| {
            (each.manifest == package.manifest && !member)
                || members.iter().any(|membership| {
                    membership.workspace == each.manifest && membership.package == package.manifest
                })
        })
        .flat_map(|each| &each.declared.overrides)
        .find(|each| &parsed(each).name == name)
        .map_or_else(Replacement::default, |each| Replacement(each.0.clone()))
}

/// The requirements of the packages, each on the package of the repository found by the normalized name or by the directory the
/// manifest names, or on none when the repository holds no such package.
#[must_use]
pub(crate) fn dependencies_of(packages: &Packages) -> Vec<Dependency> {
    let mut found = Vec::new();
    let members = members_of(packages);
    for package in &packages.packages {
        for requirement in &package.declared.requirements {
            let read = parsed(&requirement.written);
            let replacement = replacement_of(packages, &members, package, &read.name);
            for target in targets_of(packages, package, requirement, &read) {
                let (binding, name) = match (
                    &requirement.reference,
                    target.package.map(|each| &each.declared.name),
                ) {
                    (Reference::Located(_), Some(Naming::Unnamed)) => continue,
                    (Reference::Located(_), Some(Naming::Named(named)))
                        if read.name != normalized(named) =>
                    {
                        (Some(named.clone()), Some(normalized(named)))
                    }
                    // A requirement that writes no name, and no package of the repository names, has no name to give.
                    _ if read.written.0.is_empty() => (None, None),
                    _ => (Some(read.written.clone()), Some(read.name.clone())),
                };
                let dependency = Dependency {
                    source: package.manifest.clone(),
                    target: target.package.map(|each| each.manifest.clone()),
                    binding,
                    package: name,
                    origin: target.origin,
                    kind: requirement.kind,
                    condition: conditioned(&read.marker, target.condition),
                    optional: requirement.optional,
                    requirement: requirement.specifier.clone(),
                    exact: requirement.exact,
                    checkout: checkout_with(&requirement.checkout, target.checkout),
                    interpreter: requirement.interpreter.clone(),
                    replacement: replacement.clone(),
                };
                if !found.contains(&dependency) {
                    found.push(dependency);
                }
            }
        }
    }
    found
}

/// The requirements between the manifests of the run, as the rows of `package_dependency`.
#[must_use]
pub(crate) fn wirings_in(files: &[languages::FilePath], packages: &Packages) -> Wired<Wiring> {
    let rows = dependencies_of(packages)
        .into_iter()
        .filter_map(|dependency| {
            Some(Wiring {
                source: file_of(files, &dependency.source)?,
                target: match &dependency.target {
                    Some(target) => file_of(files, target)?,
                    None => languages::NO_FILE,
                },
                dependency,
            })
        })
        .collect();
    Wired {
        rows,
        refused: causes_of(packages),
    }
}

/// The revision a git URL names after the `@` that ends its path (`git+https://host/repo.git@v1.0#egg=repo` names `v1.0`); empty for
/// a URL of another scheme or one that names none. The `@` of a user before the host is not it.
#[must_use]
pub(crate) fn url_revision(url: Part<'_>) -> Revision {
    let url = url.0.trim();
    let Some(rest) = url.strip_prefix(GIT_SCHEME) else {
        return Revision::default();
    };
    let rest = rest.split('#').next().unwrap_or(rest);
    let path = rest
        .split_once(AUTHORITY)
        .map_or(rest, |(_, after)| after)
        .split_once(SEPARATOR)
        .map_or(<&str>::default(), |(_, path)| path);
    path.rsplit_once(DIRECT)
        .map_or_else(Revision::default, |(_, revision)| {
            Revision(revision.trim().into())
        })
}

/// The revision of a git repository a PEP 508 requirement names: the one its direct reference's URL names (`name @ url`).
#[must_use]
pub(crate) fn checkout_of(requirement: &Written) -> Checkout {
    Checkout {
        revision: requirement
            .0
            .split(MARKER)
            .next()
            .and_then(|head| head.split_once(DIRECT))
            .map_or_else(Revision::default, |(_, url)| url_revision(Part(url))),
        ..Checkout::default()
    }
}

/// Whether a PEP 440 version specifier admits one version only: one of its clauses is `===` or `==` with no wildcard. A conjunction
/// with such a clause admits at most that version.
#[must_use]
pub(crate) fn pep440_exactness(specifier: &Specifier) -> languages::Flag {
    let pinned = specifier.0.split(CLAUSES).any(|clause| {
        let clause = clause.trim();
        clause.starts_with(ARBITRARY_EQUAL)
            || clause.strip_prefix(EQUAL).is_some_and(|version| {
                !version.trim().is_empty() && !version.trim_end().ends_with(WILDCARD)
            })
    });
    if pinned {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// Whether a Poetry constraint admits one version only: with no alternative, one of its clauses is a bare version (`1.2.3`, which
/// Poetry reads as `==1.2.3`) or a PEP 440 pin. A caret, a tilde, a wildcard and a range admit more.
#[must_use]
pub(crate) fn poetry_exactness(specifier: &Specifier) -> languages::Flag {
    if specifier.0.contains(ALTERNATIVES) {
        return languages::Flag::Lowered;
    }
    let bare = specifier.0.split(CLAUSES).any(|clause| {
        let clause = clause.trim();
        clause
            .chars()
            .next()
            .is_some_and(|each| each.is_ascii_digit())
            && !clause.contains(WILDCARD)
    });
    if bare || pep440_exactness(specifier) == languages::Flag::Raised {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}
