//! Borrowed axis traversal; the caller supplies both prepared operand populations.

use super::types::{LEFT, PositionInput, PositionWork, RIGHT, input};
use crate::domain::{
    lowering::types::LoweredShape,
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::types::WorkCounters,
    positions::{
        service::{count, followed, in_tree, row},
        types::{Positions, Slots},
    },
    table_cells::types::TreeRows,
    table_model::{service::positions, types::Verdict},
    walks::{
        service::{allows, give, take_filter},
        types::{Admit, Marked},
    },
};
use ryo_language::domain::elements::service::{
    children, next_sibling, parent_at, previous_sibling, size_at,
};
use ryo_relations::domain::relations::types::Position;
use ryo_values::domain::{
    plan_parts::types::{Direction, Input},
    plans::types::Operation,
};

pub(super) fn axis(
    operation: &Operation,
    tree: &TreeRows<'_>,
    inputs: &[PositionInput<'_>],
    output: &LoweredShape,
    context: &mut OperatorContext<'_>,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Positions, OperatingError> {
    let Operation::Axis {
        domain,
        direction,
        id,
        domain_id,
        domain_filter,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let from = inputs
        .first()
        .copied()
        .ok_or(OperatingError::MissingInput { position: 1 })?;
    let start_column = from
        .shape
        .columns
        .get(usize::try_from(id.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(OperatingError::NotRun)?;
    let mut admit = match domain {
        Input::BoundObservation(_) => return Err(OperatingError::NotAStep),
        Input::Relation(_) if domain_filter.is_empty() => Admit::All,
        Input::Relation(_) => {
            Admit::Filter(take_filter(tree, domain_filter, context), domain_filter)
        }
        Input::Step(_) => {
            let source = inputs
                .get(1)
                .copied()
                .ok_or(OperatingError::MissingInput { position: 2 })?;
            let column = source
                .shape
                .columns
                .get(usize::try_from(domain_id.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(OperatingError::NotRun)?;
            let mut marks = vec![false; tree.elements.kind.len()];
            for at in positions(count(source.rows)) {
                counts.found(1, 1);
                if let Some(hit) = followed(tree, column, row(source.rows, at)) {
                    if let Some(mark) = marks.get_mut(usize::try_from(hit.0).unwrap_or(usize::MAX))
                    {
                        *mark = true;
                    }
                }
            }
            Admit::Marks(Marked(marks))
        }
    };
    let PositionWork { found, .. } = work;
    let mut slots = Slots::default();
    let elements = tree.elements;
    let up = |at: Position| (at.0 != 0).then(|| parent_at(elements, at));
    let mut result = Ok(());
    for at in positions(count(from.rows)) {
        counts.found(0, 1);
        let each = row(from.rows, at);
        let Some(start) = followed(tree, start_column, each) else {
            continue;
        };
        let start = in_tree(tree, start);
        found.0.clear();
        match direction {
            Direction::Parent => found.0.extend(up(start).map(|parent| parent.0)),
            Direction::Child => found
                .0
                .extend(children(elements, start).map(|child| child.0)),
            Direction::Ancestor | Direction::NearestAncestor => found.0.extend(
                std::iter::successors(up(start), |above| up(*above)).map(|ancestor| ancestor.0),
            ),
            Direction::Descendant => found
                .0
                .extend(start.0 + 1..=start.0 + size_at(elements, start).0),
            Direction::Deep => found.0.extend(
                (start.0 + 1..=start.0 + size_at(elements, start).0)
                    .filter(|below| parent_at(elements, Position(*below)) != start),
            ),
            Direction::PreviousSibling => found
                .0
                .extend(previous_sibling(elements, start).map(|sibling| sibling.0)),
            Direction::NextSibling => found
                .0
                .extend(next_sibling(elements, start).map(|sibling| sibling.0)),
        }
        for reached in found.0.iter().copied() {
            counts.found(1, 1);
            match allows(&mut admit, tree, Position(reached), context) {
                Ok(Verdict::Holds) => {}
                Ok(Verdict::Fails) => continue,
                Err(error) => {
                    result = Err(error);
                    break;
                }
            }
            slots.0.extend_from_slice(each.0);
            slots.0.push(reached);
            if *direction == Direction::NearestAncestor {
                break;
            }
        }
        if result.is_err() {
            break;
        }
    }
    give(admit, context);
    result?;
    Ok(Positions {
        width: output.width,
        slots,
    })
}
