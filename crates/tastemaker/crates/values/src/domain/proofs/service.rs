//! The `proofs` service: free functions named by verb that name, stamp and digest a version's proof, with no IO and no
//! ports.

use super::types::{NodePart, ProofParts, UseText, VersionPaths};
use super::types::{Problem, Proof};
use crate::domain::files::types::Stamp;
use crate::domain::packages::types::Release;
use crate::domain::paths::types::PathText;

/// What a version another reads through is named in its digest: `kind/package/name@version`.
#[must_use]
pub fn use_text(release: &Release) -> UseText {
    use crate::domain::packages::service::kind_word;
    UseText(
        format!(
            "{}/{}/{}@{}",
            kind_word(release.kind).0,
            release.package.0,
            release.name.0,
            release.version.0
        )
        .into(),
    )
}

/// The files of the version directory `scope` among a package's `files`, and every directory of the version, `scope`
/// first: what a version's stamp is taken over.
#[must_use]
pub fn version_paths<'a>(
    files: impl IntoIterator<Item = PathText<'a>>,
    scope: PathText<'a>,
) -> VersionPaths<'a> {
    let prefix = format!("{}/", scope.0);
    let held = files
        .into_iter()
        .filter(|file| file.0.starts_with(&prefix))
        .collect::<Vec<_>>();
    let mut directories = std::collections::BTreeSet::new();
    for file in &held {
        let mut at = file.0;
        while let Some((holder, _)) = at.rsplit_once('/')
            && holder.len() > scope.0.len()
        {
            let _: bool = directories.insert(holder);
            at = holder;
        }
    }
    VersionPaths {
        files: held,
        directories: std::iter::once(scope)
            .chain(directories.into_iter().map(PathText))
            .collect(),
    }
}

/// A version's stamp from the stamps of its files and of its directories: the size of its files, all told, and the
/// latest time any of them was modified; a file added, removed, renamed or written moves it.
#[must_use]
pub fn summary(files: &[Stamp], directories: &[Stamp]) -> Stamp {
    Stamp {
        size: crate::domain::files::types::FileSize(files.iter().map(|stamp| stamp.size.0).sum()),
        modified: files
            .iter()
            .chain(directories)
            .map(|stamp| stamp.modified)
            .max()
            .unwrap_or_default(),
    }
}

/// The parts of a node's digest: each child's name and digest, in the order given.
#[must_use]
pub fn node_parts<'a>(children: impl IntoIterator<Item = NodePart<'a>>) -> ProofParts<'a> {
    ProofParts(
        children
            .into_iter()
            .flat_map(|part| [part.name.0.as_bytes(), part.digest.0.as_bytes()])
            .collect(),
    )
}

/// The proof of a version with `problems`.
#[must_use]
pub fn proof(problems: Vec<Problem>) -> Proof {
    if problems.is_empty() {
        Proof::Proven
    } else {
        Proof::Failed(problems)
    }
}
