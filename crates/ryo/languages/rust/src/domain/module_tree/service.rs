//! The `module_tree` service: free functions named by verb that draw the module tree of a crate from the facts of its files and
//! resolve the paths a file writes against it, with no IO and no ports.

use super::constants::{
    CRATE, DIRECTORY_OWNER, GLOB, MACRO_CRATE, MODULE_SEPARATOR, SELF, SELF_TYPE, SOURCE, SUPER,
};
use super::types::{
    Body, Facts, Gate, ModDeclaration, ModulePath, ModuleText, ModuleTree, Placing, Segment, SourceCount,
    Unfollowed,
};
use crate::domain::paths::service::{directory_of, lexical_join, owns_directory, stem};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::paths::types::{Ownership, Relative};
use std::collections::{BTreeMap, BTreeSet};

/// The files declaring `module` in the tree.
fn files_for<'a>(tree: &'a ModuleTree, module: &[Segment]) -> &'a [RepoPath] {
    tree.modules
        .get(&ModulePath(module.to_vec()))
        .map_or(&[], Vec::as_slice)
}

/// The files that own the longest module prefix of `written`, seen from `module`. Paths that start outside the crate
/// (external crates, the prelude, `Self`) resolve to nothing; the compiler owns whether they are valid. Several files own one
/// prefix when the module is declared under alternate `#[cfg]`s.
#[must_use]
pub(crate) fn resolve<'a>(
    tree: &'a ModuleTree,
    module: &[Segment],
    written: &[Segment],
) -> Option<&'a [RepoPath]> {
    let mut segments = written.iter();
    let mut base: Vec<Segment> = match &**segments.next()? {
        CRATE => Vec::new(),
        SELF => module.to_vec(),
        SUPER => module.split_last()?.1.to_vec(),
        MACRO_CRATE | SELF_TYPE => return None,
        first => {
            let mut child = module.to_vec();
            child.push(Segment(first.into()));
            if !tree.modules.contains_key(&ModulePath(child)) {
                return None;
            }
            // A child module named bare: the first segment is part of the target.
            segments = written.iter();
            module.to_vec()
        }
    };
    for segment in segments {
        match &**segment {
            SUPER => base.truncate(base.len().checked_sub(1)?),
            GLOB | SELF => {}
            _ => base.push(segment.clone()),
        }
    }
    // The module, then each module above it up to the crate root, the nearest first.
    std::iter::successors(Some(base.as_slice()), |prefix| {
        prefix.split_last().map(|(_, rest)| rest)
    })
    .map(|prefix| files_for(tree, prefix))
    .find(|files| !files.is_empty())
}

/// What a file declares, as the module it declares and where that module's own children and `#[path]` are relative to, given
/// the module the file is and the directory its children resolve under.
fn placed(
    declared: &ModDeclaration,
    file: &RepoPath,
    module: &[Segment],
    base: &RepoPath,
) -> Placing {
    let (mut base, mut path_base) = (base.clone(), directory_of(file));
    let mut child = module.to_vec();
    for step in &declared.around {
        let inline = match &step.path {
            Section::Present(explicit) => lexical_join(&path_base, &Relative(explicit.0.clone())),
            Section::Absent => lexical_join(&base, &Relative(step.name.0.clone())),
        };
        base.clone_from(&inline);
        path_base = inline;
        child.push(step.name.clone());
    }
    child.push(declared.name.clone());
    Placing {
        module: ModulePath(child),
        base,
        path_base,
    }
}

/// The tree of the crate rooted at `root`, of the package in `package`, its module files found among `files`.
pub(crate) fn tree<'f>(
    files: &BTreeSet<RepoPath>,
    package: &RepoPath,
    root: &RepoPath,
    facts_of: &(dyn Fn(&RepoPath) -> Option<&'f Facts> + Sync),
) -> ModuleTree {
    let mut drawn = ModuleTree {
        package: package.clone(),
        modules: BTreeMap::new(),
        files: BTreeMap::new(),
        gaps: Vec::new(),
        tests: BTreeSet::new(),
    };
    declare(&mut drawn, ModulePath::default(), root);
    let mut active = vec![root.clone()];
    visit(
        files,
        &mut drawn,
        &mut active,
        root,
        &[],
        &directory_of(root),
        facts_of,
    );
    drawn
}

/// Declares the module `module` in `file`.
fn declare(tree: &mut ModuleTree, module: ModulePath, file: &RepoPath) {
    tree.files
        .entry(file.clone())
        .or_default()
        .push(module.clone());
    tree.modules.entry(module).or_default().push(file.clone());
}

/// Follows every module `file` declares as module `module`, its children resolving under `base`. `active` is the chain of files
/// being visited: a file declaring itself, directly or through `#[path]`, is a cycle; a file placed twice under different modules is not.
fn visit<'f>(
    files: &BTreeSet<RepoPath>,
    tree: &mut ModuleTree,
    active: &mut Vec<RepoPath>,
    file: &RepoPath,
    module: &[Segment],
    base: &RepoPath,
    facts_of: &(dyn Fn(&RepoPath) -> Option<&'f Facts> + Sync),
) {
    let Some(facts) = facts_of(file) else { return };
    for declared in &facts.declared {
        let Placing {
            module: child,
            base: child_base,
            path_base,
        } = placed(declared, file, module, base);
        if declared.gate == Gate::Test {
            tree.tests.extend([child.clone()]);
        }
        if declared.body == Body::Inline {
            tree.modules.entry(child).or_default().push(file.clone());
            continue;
        }
        let candidates = match &declared.path {
            Section::Present(explicit) => {
                vec![lexical_join(&path_base, &Relative(explicit.0.clone()))]
            }
            Section::Absent => {
                let sibling = Relative(format!("{}{SOURCE}", declared.name).into());
                let owner = Relative(format!("{}/{DIRECTORY_OWNER}", declared.name).into());
                vec![
                    lexical_join(&child_base, &sibling),
                    lexical_join(&child_base, &owner),
                ]
            }
        };
        let present: Vec<RepoPath> = candidates
            .into_iter()
            .filter(|candidate| files.contains(candidate))
            .collect();
        // A module with no file, or with two, is not followed.
        let [target] = present.as_slice() else {
            tree.gaps.push(Unfollowed::Files {
                file: file.clone(),
                module: module_text(&child),
                count: if present.is_empty() {
                    SourceCount::None
                } else {
                    SourceCount::Two
                },
            });
            continue;
        };
        declare(tree, child.clone(), target);
        if active.contains(target) {
            tree.gaps.push(Unfollowed::Loading {
                file: file.clone(),
                module: module_text(&child),
                target: target.clone(),
            });
            continue;
        }
        // A file loaded through `#[path]` or `mod.rs` owns its directory; `x.rs` owns `x/`.
        let next_base = if matches!(declared.path, Section::Present(_))
            || owns_directory(target) == Ownership::Owns
        {
            directory_of(target)
        } else {
            lexical_join(&directory_of(target), &Relative(stem(target).0))
        };
        let depth = active.len();
        active.push(target.clone());
        visit(files, tree, active, target, &child, &next_base, facts_of);
        active.truncate(depth);
    }
}

/// A module path as it is read in a finding: its segments joined with `::`.
fn module_text(module: &ModulePath) -> ModuleText {
    ModuleText(
        module
            .iter()
            .map(|segment| &**segment)
            .collect::<Vec<_>>()
            .join(MODULE_SEPARATOR)
            .into(),
    )
}
