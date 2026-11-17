//! The placed-by selector, `{ placed-by = "…" }`: the files one template entry placed beneath the place, by the entry's
//! reference (its alias, or its template's name).

use std::collections::BTreeSet;

use super::super::interface::SelectorsItem;
use crate::domain::paths::service::containment;
use crate::domain::paths::types::Containment;
use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::types::{Resolution, Resolved, SelectionContext, Selector};

/// The placed-by selector kind.
pub(crate) struct PlacedByItem;

impl SelectorsItem for PlacedByItem {
    fn resolve(
        &self,
        selector: &Selector,
        context: &SelectionContext<'_>,
    ) -> Result<Resolution, SelectorsError> {
        let Selector::PlacedBy(placed) = selector else {
            return Ok(Resolution::NotMine);
        };
        let selected = context
            .placed
            .iter()
            .filter(|file| {
                file.entry == placed.entry
                    && (containment(&file.path, context.base) == Containment::Inside)
            })
            .map(|file| file.path.clone())
            .collect::<BTreeSet<_>>();
        Ok(Resolution::Resolved(Resolved::Paths(selected)))
    }
}
