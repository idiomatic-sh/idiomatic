//! The part selector, `{ part = "…" }`: the directories of the instances of one spec beneath the place, those directly
//! beneath it, or with `recursive = true` those at any depth. It covers each directory and everything in it.

use std::collections::BTreeSet;

use super::super::interface::SelectorsItem;
use crate::domain::paths::service::containment;
use crate::domain::paths::types::Containment;
use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::types::{
    PartInstance, Recursion, Resolution, Resolved, SelectionContext, Selector,
};

/// The part selector kind.
pub(crate) struct PartItem;

impl SelectorsItem for PartItem {
    fn resolve(
        &self,
        selector: &Selector,
        context: &SelectionContext<'_>,
    ) -> Result<Resolution, SelectorsError> {
        let Selector::Part(part) = selector else {
            return Ok(Resolution::NotMine);
        };
        let beneath = |instance: &&PartInstance| {
            instance.directory != *context.base
                && (containment(&instance.directory, context.base) == Containment::Inside)
        };
        let candidates = context.instances.iter().filter(beneath).collect::<Vec<_>>();
        let selected = candidates
            .iter()
            .filter(|instance| instance.spec == part.part)
            .filter(|instance| match part.recursion {
                Recursion::Recursive => true,
                // Directly beneath: no other instance lies between the place and this one.
                Recursion::Direct => !candidates.iter().any(|other| {
                    other.directory != instance.directory
                        && (containment(&instance.directory, &other.directory)
                            == Containment::Inside)
                }),
            })
            .map(|instance| instance.directory.clone())
            .collect::<BTreeSet<_>>();
        Ok(Resolution::Resolved(Resolved::Paths(selected)))
    }
}
