//! The `globs` service: free functions named by verb that write files as the globs a rule of the lint call carries, with
//! no IO and no ports.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::types::{FileCount, FileCounts, Holder, HolderPath, Within};
use crate::domain::applications::constants::{EVERYTHING, PATH_SEPARATOR};
use crate::domain::applications::types::GlobText;
use crate::domain::paths::service::relative;
use crate::domain::paths::types::{Relative, RepoPath};

/// `path` as a glob that matches it alone.
#[must_use]
pub fn literal(path: Relative<'_>) -> GlobText {
    let mut glob = String::with_capacity(path.0.len());
    for character in path.0.chars() {
        if matches!(character, '*' | '?' | '[' | ']' | '{' | '}' | '\\') {
            glob.push('[');
            glob.push(character);
            glob.push(']');
        } else {
            glob.push(character);
        }
    }
    GlobText(glob.into())
}

/// `path` and everything beneath it, as a glob.
#[must_use]
fn beneath_glob(path: Relative<'_>) -> GlobText {
    GlobText(format!("{}/**", literal(path).0).into())
}

/// The directories holding `path`: the nearest first, the root (empty) last.
pub(crate) fn holders(path: &RepoPath) -> impl Iterator<Item = Holder<'_>> {
    let mut rest = Some(&*path.0);
    std::iter::from_fn(move || {
        let current = rest?;
        if current.is_empty() {
            rest = None;
            return None;
        }
        let parent = current
            .rsplit_once(PATH_SEPARATOR)
            .map_or_else(<&str>::default, |(parent, _)| parent);
        rest = Some(parent);
        Some(Holder(parent))
    })
}

/// How many governed files each directory holds, at any depth: what [`compress`] reads, counted once for a plan.
#[must_use]
pub fn counted(files: &[RepoPath]) -> FileCounts {
    let mut governed: HashMap<Holder<'_>, FileCount> = HashMap::new();
    for file in files {
        for directory in holders(file) {
            governed.entry(directory).or_default().0 += 1;
        }
    }
    FileCounts(
        governed
            .into_iter()
            .map(|(directory, count)| (HolderPath(directory.0.into()), count))
            .collect(),
    )
}

/// Whether `directory` is `base` or within it.
fn within(directory: Holder<'_>, base: &RepoPath) -> Within {
    if base.0.is_empty()
        || directory.0 == &*base.0
        || directory
            .0
            .strip_prefix(&*base.0)
            .is_some_and(|rest| rest.starts_with(PATH_SEPARATOR))
    {
        Within::Inside
    } else {
        Within::Outside
    }
}

/// Whether a directory holds a local package, or is one: its files are a package's, which a glob of the workspace's never
/// takes whole.
fn packaged(directory: Holder<'_>, local: &[RepoPath]) -> Within {
    if local.iter().any(|package| {
        directory.0.is_empty()
            || &*package.0 == directory.0
            || package
                .0
                .strip_prefix(directory.0)
                .is_some_and(|rest| rest.starts_with(PATH_SEPARATOR))
    }) {
        Within::Inside
    } else {
        Within::Outside
    }
}

/// The fewest globs, relative to `base`, that cover exactly `reached` of the governed files, whose counts are
/// `governed`: a directory all of whose governed files are reached, and that holds no local package, is one `<dir>/**`;
/// any other file is named alone. Each directory's reached files are counted once, so the cost is the files times their
/// depth.
#[must_use]
pub fn compress(
    reached: &BTreeSet<RepoPath>,
    governed: &FileCounts,
    local: &[RepoPath],
    base: &RepoPath,
) -> Vec<GlobText> {
    let mut hit: HashMap<Holder<'_>, FileCount> = HashMap::new();
    for file in reached {
        for directory in holders(file) {
            hit.entry(directory).or_default().0 += 1;
        }
    }
    let whole = |directory: Holder<'_>| {
        within(directory, base) == Within::Inside
            && hit.get(&directory).copied() == governed.0.get(directory.0).copied()
            && packaged(directory, local) == Within::Outside
    };
    let mut globs = Vec::new();
    let mut covered: HashSet<Holder<'_>> = HashSet::new();
    for file in reached {
        let ancestors = holders(file).collect::<Vec<_>>();
        if ancestors
            .iter()
            .any(|directory| covered.contains(directory))
        {
            continue;
        }
        match ancestors
            .into_iter()
            .rev()
            .find(|directory| whole(*directory))
        {
            Some(directory) => {
                match relative(&RepoPath(directory.0.into()), base) {
                    Some(rest) if rest.0.is_empty() => globs.push(GlobText(EVERYTHING.into())),
                    Some(rest) => globs.push(beneath_glob(rest)),
                    None => {}
                }
                let _: bool = covered.insert(directory);
            }
            None => {
                if let Some(rest) = relative(file, base) {
                    globs.push(literal(rest));
                }
            }
        }
    }
    globs
}
