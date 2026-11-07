//! The `carrying` service: free functions named by verb that hand ryo whole the releases whose applications are apart,
//! each as one rule carrying its entries' globs, with no IO and no ports.

use std::collections::BTreeMap;

use super::types::{CarriedSplit, HandedKey, Overlap, RuleGlobs};
use crate::domain::applications::constants::{CARRIED_GLOBS, EVERYTHING};
use crate::domain::applications::types::{Carried, GlobText, HandedRule, Nameable, ParamSet};
use crate::domain::entries::service::identity;
use crate::domain::globs::service::literal;
use crate::domain::packages::types::{DefinitionKind, Release};
use crate::domain::paths::service::{containment, relative};
use crate::domain::paths::types::{Containment, RepoPath};
use crate::domain::placement::errors::PlacementError;
use crate::domain::placement::types::Placement;
use crate::domain::selectors::service::resolve;
use crate::domain::selectors::types::{
    GlobForm, MatcherShape, PatternCompiler, Resolved, ResolvedSelectors, SelectionContext,
};

/// The applications of the releases ryo can be handed whole, as one rule per release and params carrying their entries'
/// globs, and the applications whose files are matched here. A release is handed whole when no file is reached by two of
/// its applications and every selector of each is a glob that decides alone; its
/// entries' `exclude` become the rule's `exclude`, which, the applications being apart, excludes nothing another entry
/// includes.
///
/// # Errors
///
/// `Selector` when an entry's paths do not compile.
pub fn carried<'w>(
    placement: &Placement,
    applications: &[Carried<'w>],
    known: &Nameable,
    compile: &dyn PatternCompiler,
    base: &RepoPath,
) -> Result<CarriedSplit<'w>, PlacementError> {
    let mut by_release: BTreeMap<&Release, Vec<&Carried<'w>>> = BTreeMap::new();
    for application in applications {
        by_release
            .entry(&application.release)
            .or_default()
            .push(application);
    }
    let mut direct: BTreeMap<HandedKey, RuleGlobs> = BTreeMap::new();
    let mut matched = Vec::new();
    for (release, carried) in by_release {
        if reach(placement, &carried) == Overlap::Shared {
            matched.extend(carried.into_iter().cloned());
            continue;
        }
        // The globs a rule would carry, counted from the selectors before any is resolved: two for each selector.
        let mut estimate: BTreeMap<&ParamSet, usize> = BTreeMap::new();
        for application in &carried {
            let header = &application.entry.header;
            *estimate.entry(&application.params).or_default() +=
                2 * (header.include.0.len() + header.exclude.0.len());
        }
        if estimate.values().any(|count| *count > CARRIED_GLOBS) {
            matched.extend(carried.into_iter().cloned());
            continue;
        }
        let globs = carried
            .iter()
            .map(|application| handed(placement, application, known, compile, base))
            .collect::<Result<Option<Vec<_>>, _>>()?;
        let Some(globs) = globs else {
            matched.extend(carried.into_iter().cloned());
            continue;
        };
        // A rule carrying many globs costs ryo more to match than the files it reaches cost here: such a release is
        // matched here, and ryo is handed the files.
        let mut carrying: BTreeMap<&ParamSet, usize> = BTreeMap::new();
        for (application, rule) in carried.iter().zip(&globs) {
            *carrying.entry(&application.params).or_default() +=
                rule.include.len() + rule.exclude.len();
        }
        if carrying.values().any(|count| *count > CARRIED_GLOBS) {
            matched.extend(carried.into_iter().cloned());
            continue;
        }
        for (application, rule) in carried.into_iter().zip(globs) {
            // An entry whose selectors select nothing (`include = []`, a part with no instance, a placed-by with no file)
            // reaches no file, as one whose glob matches none: it hands ryo no rule, since a rule with no `include` reads
            // every file.
            if rule.include.is_empty() {
                continue;
            }
            let slot = direct
                .entry(HandedKey {
                    release: release.clone(),
                    params: application.params.clone(),
                })
                .or_default();
            slot.include.extend(rule.include);
            slot.exclude.extend(rule.exclude);
        }
    }
    let handed = direct
        .into_iter()
        .map(|(key, mut rule)| {
            rule.include.sort();
            rule.include.dedup();
            rule.exclude.sort();
            rule.exclude.dedup();
            HandedRule {
                release: key.release,
                params: key.params,
                include: rule.include,
                exclude: rule.exclude,
            }
        })
        .collect();
    Ok(CarriedSplit { handed, matched })
}

/// Whether two of the applications can reach one file: one's directory is another's or lies within it, as a scope's
/// rules read its nested instances too.
fn reach(placement: &Placement, applications: &[&Carried<'_>]) -> Overlap {
    let dirs = applications
        .iter()
        .filter_map(|application| placement.scopes.get(application.scope.0))
        .map(|scope| &scope.dir)
        .collect::<Vec<_>>();
    let shared = dirs.iter().enumerate().any(|(position, inner)| {
        dirs.iter().enumerate().any(|(other, outer)| {
            other != position
                && (inner == outer || containment(inner, outer) == Containment::Inside)
        })
    });
    if shared {
        Overlap::Shared
    } else {
        Overlap::Apart
    }
}

/// One application's `include` and `exclude` as globs relative to `base`; none when a selector is not a glob that
/// decides alone.
///
/// # Errors
///
/// `Selector` when an entry's paths do not compile.
#[expect(
    clippy::indexing_slicing,
    reason = "a scope, an entry or a template index is made by place_at over these scopes, compositions and loaded definitions: an index past them is a bug that must crash, never a skipped reach, #count, #closed or #coverage finding"
)]
fn handed(
    placement: &Placement,
    application: &Carried<'_>,
    known: &Nameable,
    compile: &dyn PatternCompiler,
    base: &RepoPath,
) -> Result<Option<RuleGlobs>, PlacementError> {
    let scope = &placement.scopes[application.scope.0];
    let context = SelectionContext {
        base: &scope.dir,
        values: &scope.values,
        instances: &known.instances,
        placed: &known.placed,
        compile,
    };
    let entry = application.entry;
    let failed = |source| PlacementError::Selector {
        entry: identity(&entry.header, DefinitionKind::Rule, &entry.rule).0,
        source,
    };
    let include = resolve(&entry.header.include, &context).map_err(failed)?;
    let exclude = resolve(&entry.header.exclude, &context).map_err(failed)?;
    let (Some(include), Some(exclude)) = (globs_of(&include, base), globs_of(&exclude, base))
    else {
        return Ok(None);
    };
    Ok(Some(RuleGlobs { include, exclude }))
}

/// A glob and everything beneath what it matches, as a selector list covers a file a directory it matches holds.
fn both(globs: &mut Vec<GlobText>, glob: GlobText) {
    globs.push(GlobText(format!("{}/**", glob.0).into()));
    globs.push(glob);
}

/// A resolved selector list as globs relative to `base`: each pattern and everything beneath what it matches, as the
/// list covers a file a directory it matches holds; none when an item is not a glob that decides alone.
fn globs_of(resolved: &ResolvedSelectors, base: &RepoPath) -> Option<Vec<GlobText>> {
    let mut globs = Vec::new();
    for item in &resolved.0 {
        match item {
            Resolved::Pattern(matcher) => {
                let at = relative(&matcher.base, base)?;
                match &matcher.shape {
                    MatcherShape::Itself => {
                        if at.0.is_empty() {
                            globs.push(GlobText(EVERYTHING.into()));
                        } else {
                            both(&mut globs, literal(at));
                        }
                    }
                    MatcherShape::Compiled(compiled) => match &compiled.glob {
                        GlobForm::Alone(glob) => {
                            let prefixed = if at.0.is_empty() {
                                glob.0.to_string()
                            } else {
                                format!("{}/{}", literal(at).0, glob.0)
                            };
                            both(&mut globs, GlobText(prefixed.into()));
                        }
                        GlobForm::Bound => return None,
                    },
                }
            }
            Resolved::Paths(paths) => {
                for path in paths {
                    both(&mut globs, literal(relative(path, base)?));
                }
            }
        }
    }
    Some(globs)
}
