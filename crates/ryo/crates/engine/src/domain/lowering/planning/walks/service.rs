//! The services of the `walks` part: a walk along a tree axis and a nearest walk, as accesses.

use crate::domain::lowering::costing::types::CellState;
use crate::domain::lowering::definitions::service::{asked, fresh_register};
use crate::domain::lowering::flattening::types::FlatTerm;
use crate::domain::lowering::planning::operands::service::{
    given_or_tested, register_of, state_of,
};
use crate::domain::lowering::planning::types::{Emitter, Enumerated, UnitRole, WrittenAccess};
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{
    AxisDirection, ColumnMask, DefinitionId, MadeColumn, RegisterList, StepOperand, StepOperands,
};
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
    ends: (FlatTerm, FlatTerm),
) {
    let given =
        |emitter: &Emitter<'_>, term| state_of(&emitter.environment, term) == CellState::Register;
    let (from_start, from_reached) = (given(emitter, ends.0), given(emitter, ends.1));
    emitter.environment.enumerated = Enumerated::Already;
    let contained = match direction {
        AxisDirection::Ancestor => Some((ends.1, ends.0)),
        AxisDirection::Descendant => Some((ends.0, ends.1)),
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
            (None, _) | (_, None) => emitter.missed = emitter.missed.saturating_add(1),
        }
        return;
    }
    // From the start, along the axis; from what is reached, back along it: a parent's children are a walk of their own.
    let (walk, from, to) = if from_start {
        (Some(direction), ends.0, ends.1)
    } else {
        (reversed(direction), ends.1, ends.0)
    };
    let Some(start) = register_of(emitter, from) else {
        emitter.missed = emitter.missed.saturating_add(1);
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
    walk: (Relation, DefinitionId, MadeColumn),
    start: FlatTerm,
    reached: FlatTerm,
) {
    let Some(from) = register_of(emitter, start) else {
        emitter.missed = emitter.missed.saturating_add(1);
        return;
    };
    emitter.environment.enumerated = Enumerated::Already;
    let out = fresh_register(emitter.drafts);
    let column_of = |emitter: &Emitter<'_>, term: FlatTerm| {
        let place = emitter.row.iter().position(|each| *each == term)?;
        u16::try_from(place).ok().map(MadeColumn)
    };
    match (emitter.role, emitter.unit) {
        (UnitRole::Pipeline, Some(unit)) => {
            let (Some(bound), Some(collected)) =
                (column_of(emitter, start), column_of(emitter, reached))
            else {
                emitter.missed = emitter.missed.saturating_add(1);
                return;
            };
            let (bound, collected) = (ColumnMask([bound].into()), ColumnMask([collected].into()));
            asked(emitter.drafts, unit, &bound, &collected);
            emitter.written.push(WrittenAccess::Use {
                definition: unit,
                bound,
                collected,
                values: StepOperands([StepOperand::Register(from)].into()),
                outputs: RegisterList([out].into()),
                mode: UseMode::Rows,
            });
        }
        (UnitRole::Branch | UnitRole::Pattern | UnitRole::Pipeline, _) => {
            let bound = ColumnMask([walk.2].into());
            asked(emitter.drafts, walk.1, &bound, &ColumnMask::default());
            let element = fresh_register(emitter.drafts);
            emitter.written.push(WrittenAccess::Nearest {
                relation: walk.0,
                domain: walk.1,
                column: walk.2,
                element,
                start: from,
                out,
            });
        }
    }
    given_or_tested(emitter, reached, out);
}
