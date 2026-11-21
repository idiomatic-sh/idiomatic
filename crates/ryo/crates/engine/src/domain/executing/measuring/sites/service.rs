//! The services of the `sites` part: what a measuring worker does when a file's scan ends. The rows of each size site of
//! the code are counted, through the seam, and then the file's counts are folded (`measuring::service::file_ended`). It
//! is apart from the hooks because it asks the fill side, which the hooks' own file must not: the loop's answers to the
//! fill side call the hooks.

use crate::domain::code::types::{ConstantNumber, RelationNumber};
use crate::domain::executing::measuring::service::{file_ended, slot_mut};
use crate::domain::executing::types::Asker;
use crate::domain::indexes::reading::service::cell_at;
use crate::domain::made_tables::reading::service::view_of;
use crate::domain::made_tables::sources::service::scanned;
use crate::domain::made_tables::types::ReadSource;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::constant;
use ryo_relations::domain::relations::types::Count;

/// Counts the rows of each size site of the code in the file: the rows of a filled relation that hold one constant in
/// one column. The relation is filled whole for it, so every site is counted in every file a measuring worker scans.
fn literals_counted(frame: &mut StepFrame<'_, '_>) {
    let code = frame.fill.programs.code;
    for (place, site) in code.sizes.iter().enumerate() {
        let source = ReadSource::Filled(RelationNumber(site.relation.0));
        let wanted = constant(frame, ConstantNumber(site.value.0));
        let asked = scanned(
            frame.fill,
            &mut Asker {
                state: &mut *frame.state,
                found: &mut *frame.found,
                ports: frame.ports,
            },
            source,
        );
        let Ok(whole) = asked else {
            continue;
        };
        // The column is asked once for the site, and its cells are read where they lie.
        let column = view_of(frame.fill.store, source, site.column);
        let rows = (whole.rows.start.0..whole.rows.end.0)
            .filter(|row| cell_at(column, Count(*row)) == wanted)
            .count();
        if let Some(measure) = frame.state.measure.as_deref_mut()
            && let Some(held) = slot_mut(&mut measure.literals, Count(place))
        {
            *held = Count(held.0.saturating_add(rows));
        }
    }
}

/// The file's scan ends: in a measuring worker the size sites' rows are counted and the file's counts folded; a plain
/// worker does nothing.
pub(crate) fn file_measured(frame: &mut StepFrame<'_, '_>) {
    if frame.state.measure.is_some() {
        literals_counted(frame);
        file_ended(frame);
    }
}
