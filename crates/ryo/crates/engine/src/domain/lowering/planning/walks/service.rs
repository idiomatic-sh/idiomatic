//! The services of the `walks` part: a walk along a tree axis and a nearest walk, as accesses.

use crate::domain::lowering::costing::types::CellState;
use crate::domain::lowering::definitions::service::{asked, fresh_register};
use crate::domain::lowering::flattening::types::{NearestDomain, NearestOver};
use crate::domain::lowering::planning::closures::service::{own_used, read_condition, walk_edges};
use crate::domain::lowering::planning::closures::types::{AtomWriter, EdgeEnds};
use crate::domain::lowering::planning::operands::service::{
    given_or_tested, register_of, state_of,
};
use crate::domain::lowering::planning::types::{EdgeUse, NearestWalked, WalkEdges};
use crate::domain::lowering::planning::types::{Emitter, Enumerated, UnitRole, WrittenAccess};
use crate::domain::lowering::planning::walks::types::{NearestClimb, WalkEnds};
use crate::domain::steps::types::{AxisDirection, ColumnMask, StepOperand, StepOperands};
use ryo_relations::domain::relations::types::Count;
use ryo_relations::domain::relations::types::Relation;

/// The axis that walks an axis back: from what it reaches to what it starts from.
pub(crate) const fn reversed(direction: AxisDirection) -> Option<AxisDirection> {
    match direction {
        AxisDirection::Ancestor => Some(AxisDirection::Descendant),
        AxisDirection::Descendant => Some(AxisDirection::Ancestor),
        AxisDirection::NextSibling => Some(AxisDirection::PreviousSibling),
        AxisDirection::PreviousSibling => Some(AxisDirection::NextSibling),
        // A parent's children are a walk of their own (`Children`): no axis walks to them.
        AxisDirection::Parent => None,
    }
}

/// A walk along a tree axis. From the end that is given it enumerates the other; between two given elements a program
/// tests the containment in place, and a pipeline walks from the start and keeps the rows that reach the other.
pub(crate) fn walked(
    emitter: &mut Emitter<'_>,
    relation: Relation,
    direction: AxisDirection,
    ends: WalkEnds,
) {
    let given =
        |emitter: &Emitter<'_>, term| state_of(&emitter.environment, term) == CellState::Register;
    let (from_start, from_reached) = (given(emitter, ends.start), given(emitter, ends.reached));
    emitter.environment.enumerated = Enumerated::Already;
    let contained = match direction {
        AxisDirection::Ancestor => Some((ends.reached, ends.start)),
        AxisDirection::Descendant => Some((ends.start, ends.reached)),
        AxisDirection::Parent | AxisDirection::NextSibling | AxisDirection::PreviousSibling => None,
    };
    if let (true, true, Some((ancestor, descendant)), UnitRole::Branch | UnitRole::Pattern) =
        (from_start, from_reached, contained, emitter.role)
    {
        let registers = (
            register_of(emitter, ancestor),
            register_of(emitter, descendant),
        );
        match registers {
            (Some(ancestor), Some(descendant)) => emitter.written.push(WrittenAccess::Within {
                relation,
                ancestor,
                descendant,
            }),
            (None, _) | (_, None) => emitter.missed = Count(emitter.missed.0.saturating_add(1)),
        }
        return;
    }
    // From the start, along the axis; from what is reached, back along it: a parent's children are a walk of their own.
    let (walk, from, to) = if from_start {
        (Some(direction), ends.start, ends.reached)
    } else {
        (reversed(direction), ends.reached, ends.start)
    };
    let Some(start) = register_of(emitter, from) else {
        emitter.missed = Count(emitter.missed.0.saturating_add(1));
        return;
    };
    let out = fresh_register(emitter.drafts);
    emitter.written.push(match walk {
        Some(direction) => WrittenAccess::Axis {
            relation,
            direction,
            start,
            out,
        },
        None => WrittenAccess::Children {
            relation,
            parent: start,
            out,
        },
    });
    given_or_tested(emitter, to, out);
}

/// A nearest walk. A program runs it in place, the domain's definition asked whether it holds the element under test. A
/// pipeline holds no walk with a condition: it uses the pattern of its own definition that binds the start and collects
/// what is reached, whose program is this walk, run once per distinct start.
pub(crate) fn nearest(
    emitter: &mut Emitter<'_>,
    walk: &NearestClimb,
    ends: WalkEnds,
    write: AtomWriter,
) {
    let (start, reached) = (ends.start, ends.reached);
    let Some(from) = register_of(emitter, start) else {
        emitter.missed = Count(emitter.missed.0.saturating_add(1));
        return;
    };
    emitter.environment.enumerated = Enumerated::Already;
    let out = fresh_register(emitter.drafts);
    match (emitter.role, emitter.unit) {
        (UnitRole::Pipeline, Some(unit)) => {
            if own_used(emitter, unit, &[start], reached, out).is_none() {
                return;
            }
        }
        (UnitRole::Branch | UnitRole::Pattern | UnitRole::Pipeline, _) => {
            let edges = match &walk.over {
                NearestOver::Tree(relation) => Some(NearestWalked::Tree(*relation)),
                // The element whose parent is asked is written into its own register before the edges run.
                NearestOver::Derived(edges) => {
                    let hand = fresh_register(emitter.drafts);
                    let asks = EdgeEnds {
                        asked: edges.source,
                        gives: edges.destination,
                    };
                    walk_edges(emitter, edges, &[], asks, hand, write)
                        .map(|edges| NearestWalked::Derived { edges, from: hand })
                }
            };
            let Some(edges) = edges else {
                emitter.missed = Count(emitter.missed.0.saturating_add(1));
                return;
            };
            let element = fresh_register(emitter.drafts);
            let condition = match &walk.domain {
                NearestDomain::Held { definition, column } => {
                    let bound = ColumnMask([*column].into());
                    asked(emitter.drafts, *definition, &bound, &ColumnMask::default());
                    Some(WalkEdges::Use(EdgeUse {
                        definition: *definition,
                        bound,
                        collected: ColumnMask::default(),
                        values: StepOperands([StepOperand::Register(element)].into()),
                    }))
                }
                NearestDomain::Read { atoms, entity } => {
                    read_condition(emitter, atoms, *entity, element, write)
                }
            };
            let Some(condition) = condition else {
                emitter.missed = Count(emitter.missed.0.saturating_add(1));
                return;
            };
            emitter.written.push(WrittenAccess::Nearest {
                edges,
                condition,
                element,
                start: from,
                out,
            });
        }
    }
    given_or_tested(emitter, reached, out);
}
