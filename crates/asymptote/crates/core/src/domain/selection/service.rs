//! The `selection` service: which violations the selectors choose, with no IO and no ports.

use super::constants::NEAREST;
use super::types::{At, Choice, PathMatch, Selectors};
use crate::domain::violations::service::anchor;
use crate::domain::violations::types::Violation;

/// Whether `selectors` choose `violation`, its path already matched against their patterns.
#[must_use]
pub fn chosen(violation: &Violation, selectors: &Selectors, path: PathMatch) -> Choice {
    let tool = selectors.tools.is_empty() || selectors.tools.contains(&violation.tool);
    let rule = selectors.rules.is_empty() || selectors.rules.contains(&violation.rule);
    let at = selectors.at.is_empty()
        || selectors
            .at
            .iter()
            .any(|at| located(violation, at) == Choice::Chosen);
    if tool && rule && at && path == PathMatch::Matched {
        Choice::Chosen
    } else {
        Choice::Left
    }
}

/// Whether `violation` is at `at`.
#[must_use]
pub(crate) fn located(violation: &Violation, at: &At) -> Choice {
    if violation.path == at.path && anchor(violation.place) == at.line {
        Choice::Chosen
    } else {
        Choice::Left
    }
}

/// The locations that no violation of `chosen` is at.
#[must_use]
pub fn unmatched<'a>(chosen: &[Violation], at: &'a [At]) -> Vec<&'a At> {
    at.iter()
        .filter(|at| {
            !chosen
                .iter()
                .any(|violation| located(violation, at) == Choice::Chosen)
        })
        .collect()
}

/// The new violations nearest `at`: those of its file by distance from its line, or the first ones when its file has
/// none.
#[must_use]
pub fn nearest(new: &[Violation], at: &At) -> Vec<Violation> {
    let mut same: Vec<&Violation> = new
        .iter()
        .filter(|violation| violation.path == at.path)
        .collect();
    if same.is_empty() {
        return new.iter().take(NEAREST).cloned().collect();
    }
    same.sort_by_key(|violation| anchor(violation.place).0.abs_diff(at.line.0));
    same.into_iter().take(NEAREST).cloned().collect()
}
