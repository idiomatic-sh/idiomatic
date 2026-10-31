//! The `manifest` service: the groups of tools and which of them a check runs, and the products a ratchet command
//! reaches, with no IO and no ports.

use super::constants::{HOOKS, LISTED_WITH, ROOT_PATH};
use super::types::{
    Group, Groups, Hook, HookList, HookName, HookWord, Layout, PreRelease, PreReleaseTag,
    ProductPath, ProductRoot, Reach, Reached, Release, Running, Satisfaction, Tool, Version,
    VersionPart,
};
use crate::domain::violations::constants::SEPARATOR;
use crate::domain::violations::types::ToolId;

/// The groups `words` name: every tool's when none is named, otherwise those, each once.
#[must_use]
pub fn groups(words: &[Group]) -> Groups {
    if words.is_empty() {
        return Groups::All;
    }
    let mut named = Vec::with_capacity(words.len());
    for word in words {
        if !named.contains(word) {
            named.push(word.clone());
        }
    }
    Groups::Only(named)
}

/// Whether a check of `selected` runs the tools of `group`.
#[must_use]
pub fn selects(selected: &Groups, group: &Group) -> Running {
    match selected {
        Groups::Only(only) if !only.contains(group) => Running::Skips,
        Groups::All | Groups::Only(_) => Running::Runs,
    }
}

/// The git hooks that exist, listed for a refusal to name.
#[must_use]
pub fn known_hooks() -> HookList {
    HookList(HOOKS.join(LISTED_WITH).into())
}

/// The hook `word` names, when it is one a git client runs.
#[must_use]
pub fn hook(word: &HookWord) -> Option<HookName> {
    HOOKS.contains(&&*word.0).then(|| HookName(word.0.clone()))
}

/// The hooks `table` maps, listed for a refusal to name.
#[must_use]
pub fn mapped(table: &[Hook]) -> HookList {
    HookList(
        table
            .iter()
            .map(|hook| &*hook.name.0)
            .collect::<Vec<_>>()
            .join(LISTED_WITH)
            .into(),
    )
}

/// The groups the hook `word` runs, when `table` maps it.
#[must_use]
pub fn hooked(table: &[Hook], word: &HookWord) -> Option<Groups> {
    table
        .iter()
        .find(|hook| *hook.name.0 == *word.0)
        .map(|hook| Groups::Only(hook.groups.clone()))
}

/// The first id two of `tools` share, when two do: a tool a capture expands into can take an id another tool has.
#[must_use]
pub fn repeated(tools: &[Tool]) -> Option<&ToolId> {
    let mut seen = std::collections::BTreeSet::new();
    tools
        .iter()
        .map(|tool| &tool.id)
        .find(|id| !seen.insert(*id))
}

/// Whether the installed version meets a pin: a full `MAJOR.MINOR.PATCH` is that version exactly, its pre-release
/// included; with a leading `^`, or with fewer parts, it is Cargo's caret requirement, which any later version up to
/// the next leftmost-nonzero part meets. Versions compare as numbers, never as text, and a pin that is no version is met
/// by no installed one. A pre-release is below its release, as Semantic Versioning orders them, and an installed
/// pre-release meets a caret pin only when the pin names a pre-release of the same `MAJOR.MINOR.PATCH`; a pre-release
/// on a pin of fewer than three parts, or an empty one, makes the pin no version; build metadata after `+` is not
/// compared.
#[must_use]
pub fn pinned(pin: &Version, installed: &Version) -> Satisfaction {
    let (caret, bare) = match pin.0.strip_prefix('^') {
        Some(rest) => (true, rest),
        None => (false, pin.0.strip_prefix('=').unwrap_or(&pin.0)),
    };
    let (Some(wanted), Some(held)) = (release(&Version(bare.into())), release(installed)) else {
        return Satisfaction::Unmet;
    };
    let given = wanted.numbers.len();
    if given == 0_usize || given > 3_usize || held.numbers.len() != 3_usize {
        return Satisfaction::Unmet;
    }
    if matches!(wanted.pre, PreRelease::Named(_)) && given != 3_usize {
        return Satisfaction::Unmet;
    }
    let part = |at| wanted.numbers.get(at).copied().unwrap_or_default();
    let floor = [part(0), part(1), part(2)];
    let [major, minor, patch] = floor.map(|number| number.0);
    if !caret && given == 3_usize {
        return if held.numbers == floor && held.pre == wanted.pre {
            Satisfaction::Met
        } else {
            Satisfaction::Unmet
        };
    }
    if let PreRelease::Named(held_pre) = &held.pre {
        // An installed pre-release meets only a pin of a pre-release of its own version, at or below it.
        return match &wanted.pre {
            PreRelease::Named(wanted_pre)
                if held.numbers == floor
                    && precedence(held_pre, wanted_pre) != std::cmp::Ordering::Less =>
            {
                Satisfaction::Met
            }
            PreRelease::Named(_) | PreRelease::Absent => Satisfaction::Unmet,
        };
    }
    let ceiling = if major > 0 {
        [major + 1, 0, 0]
    } else if given == 1 {
        [1, 0, 0]
    } else if minor > 0 {
        [0, minor + 1, 0]
    } else if given == 2 {
        [0, 1, 0]
    } else {
        [0, 0, patch + 1]
    }
    .map(VersionPart);
    // A release is above every pre-release of itself, so a release at the floor meets a pin of its pre-release.
    if held.numbers.as_slice() >= floor.as_slice() && held.numbers.as_slice() < ceiling.as_slice() {
        Satisfaction::Met
    } else {
        Satisfaction::Unmet
    }
}

/// A version's numeric parts and its pre-release, the text between the first `-` and any `+`; `None` when a part is no
/// number or the pre-release is empty.
fn release(version: &Version) -> Option<Release> {
    let written = version.0.split('+').next()?;
    let (numbers, pre) = match written.split_once('-') {
        Some((numbers, pre)) if !pre.is_empty() => {
            (numbers, PreRelease::Named(PreReleaseTag(pre.into())))
        }
        Some(_) => return None,
        None => (written, PreRelease::Absent),
    };
    let numbers = numbers
        .split('.')
        .map(|part| part.parse().map(VersionPart).ok()) // ignore: idiomatic.arch/no-ok-on-result - a part that is no number makes the text no version, which is answered by no match
        .collect::<Option<Vec<_>>>()?;
    Some(Release { numbers, pre })
}

/// How two pre-releases of one version order, as Semantic Versioning 2.0.0 orders them: identifier by identifier, a
/// numeric one below an alphanumeric one, numbers as numbers, text as ASCII, and a shorter list below a longer one it
/// begins.
fn precedence(left: &PreReleaseTag, right: &PreReleaseTag) -> std::cmp::Ordering {
    let (mut left, mut right) = (left.0.split('.'), right.0.split('.'));
    loop {
        let order = match (left.next(), right.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(one), Some(other)) => {
                match (one.parse().map(VersionPart), other.parse().map(VersionPart)) {
                    (Ok(first), Ok(second)) => first.cmp(&second),
                    (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                    (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                    (Err(_), Err(_)) => one.cmp(other),
                }
            }
        };
        if order != std::cmp::Ordering::Equal {
            return order;
        }
    }
}

/// The products a ratchet command reaches at `root`, each served by what `serve` builds for its root: `root` alone when
/// it names no `members`; otherwise the root first when it has tools of its own (`tools`, its manifest's), then each
/// member, each with its path from `root` (`.` for the root itself).
pub fn reach<S>(
    root: &ProductRoot,
    members: Vec<ProductRoot>,
    tools: &[Tool],
    mut serve: impl FnMut(&ProductRoot) -> S,
) -> Reach<S> {
    let mut reached = |path: ProductPath, product: ProductRoot| Reached {
        path,
        service: serve(&product),
        root: product,
    };
    if members.is_empty() {
        return Reach {
            layout: Layout::Product,
            products: vec![reached(ProductPath(ROOT_PATH.into()), root.clone())],
        };
    }
    let mut products = Vec::new();
    if !tools.is_empty() {
        products.push(reached(ProductPath(ROOT_PATH.into()), root.clone()));
    }
    let base = std::path::Path::new(&*root.0);
    for member in members {
        let path = std::path::Path::new(&*member.0)
            .strip_prefix(base)
            .map_or_else(
                |_| member.0.clone(),
                // Separated by `/` on every platform: the path is the member's name in the output and the lock.
                |path| {
                    let mut joined = String::new();
                    for part in path.components() {
                        if !joined.is_empty() {
                            joined.push(SEPARATOR);
                        }
                        joined.push_str(&part.as_os_str().to_string_lossy());
                    }
                    joined.into()
                },
            );
        products.push(reached(ProductPath(path), member));
    }
    Reach {
        layout: Layout::Workspace,
        products,
    }
}
