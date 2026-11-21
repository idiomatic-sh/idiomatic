//! The services of the `closures` part: a closure's walk as an access. A program runs it in place: from the source over the
//! edges, or from what is reached back over them, the edges' definition asked by the element in hand. A pipeline holds no
//! walk: it uses the pattern of its own definition that binds the keys and the seed and collects the other end, whose
//! program is this walk, run once per distinct seed.

use crate::domain::lowering::costing::types::CellState;
use crate::domain::lowering::definitions::service::{asked, fresh_register};
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::types::{EdgeColumns, FlatTerm, RowColumn};
use crate::domain::lowering::planning::operands::service::{
    given_or_tested, operand, register_of, state_of,
};
use crate::domain::lowering::planning::types::{
    BoundCell, EdgeUse, Emitter, Enumerated, UnitRole, WalkEnds, WrittenAccess,
};
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, MadeColumn, Register, RegisterList, StepOperand, StepOperands,
};
use ryo_relations::domain::relations::types::Count;

/// A use by key from its bound cells, in any order: the columns ascending, each with its cell.
fn edge_use(definition: DefinitionId, mut cells: Vec<BoundCell>, collected: MadeColumn) -> EdgeUse {
    cells.sort_by_key(|cell| cell.column);
    EdgeUse {
        definition,
        bound: ColumnMask(cells.iter().map(|cell| cell.column).collect()),
        collected: ColumnMask([collected].into()),
        values: StepOperands(cells.into_iter().map(|cell| cell.value).collect()),
    }
}

/// The column of the unit's own row that holds a term.
fn row_column(emitter: &Emitter<'_>, term: FlatTerm) -> Option<MadeColumn> {
    let place = emitter.row.iter().position(|each| *each == term)?;
    made_column(RowColumn(place))
}

/// The walk as a use of the unit's own definition: the keys and the seed bound, the other end collected.
fn own_use(
    emitter: &Emitter<'_>,
    unit: DefinitionId,
    given: &[FlatTerm],
    other: FlatTerm,
) -> Option<EdgeUse> {
    let mut cells = Vec::with_capacity(given.len());
    for term in given {
        cells.push(BoundCell {
            column: row_column(emitter, *term)?,
            value: operand(&emitter.environment, *term)?,
        });
    }
    Some(edge_use(unit, cells, row_column(emitter, other)?))
}

/// Writes a walk a pipeline cannot hold as a use of the unit's own definition: the pattern that binds the given terms'
/// columns and collects the other end's, whose program is the walk itself. The one writer of that use: the nearest walk and
/// the closure both call it. False, with the miss counted, when a term is no column of the unit's row.
pub(crate) fn own_used(
    emitter: &mut Emitter<'_>,
    unit: DefinitionId,
    given: &[FlatTerm],
    other: FlatTerm,
    out: Register,
) -> bool {
    let Some(used) = own_use(emitter, unit, given, other) else {
        emitter.missed = Count(emitter.missed.0.saturating_add(1));
        return false;
    };
    asked(emitter.drafts, unit, &used.bound, &used.collected);
    emitter.written.push(WrittenAccess::Use {
        definition: used.definition,
        bound: used.bound,
        collected: used.collected,
        values: used.values,
        outputs: RegisterList([out].into()),
        mode: UseMode::Rows,
    });
    true
}

/// Writes a closure's walk. Entered from its source it gives what is reached; entered from what is reached it gives the
/// sources, over the edges read from their destination. An end that was given already is tested against the walk's.
pub(crate) fn closed(
    emitter: &mut Emitter<'_>,
    edges: &EdgeColumns,
    keys: &[FlatTerm],
    ends: WalkEnds,
) {
    let forward = state_of(&emitter.environment, ends.start) == CellState::Register;
    let (seed_term, other) = if forward {
        (ends.start, ends.reached)
    } else {
        (ends.reached, ends.start)
    };
    let (asked_by, gives) = if forward {
        (edges.source, edges.destination)
    } else {
        (edges.destination, edges.source)
    };
    let Some(seed) = register_of(emitter, seed_term) else {
        emitter.missed = Count(emitter.missed.0.saturating_add(1));
        return;
    };
    emitter.environment.enumerated = Enumerated::Already;
    let out = fresh_register(emitter.drafts);
    if let (UnitRole::Pipeline, Some(unit)) = (emitter.role, emitter.unit) {
        let given: Vec<FlatTerm> = keys.iter().copied().chain([seed_term]).collect();
        if !own_used(emitter, unit, &given, other, out) {
            return;
        }
    } else {
        // The element whose edges are asked is written into `from` before the edges run.
        let from: Register = fresh_register(emitter.drafts);
        let mut cells = vec![BoundCell {
            column: asked_by,
            value: StepOperand::Register(from),
        }];
        for (column, key) in edges.keys.iter().zip(keys) {
            let Some(value) = operand(&emitter.environment, *key) else {
                emitter.missed = Count(emitter.missed.0.saturating_add(1));
                return;
            };
            cells.push(BoundCell {
                column: *column,
                value,
            });
        }
        let used = edge_use(edges.definition, cells, gives);
        asked(
            emitter.drafts,
            used.definition,
            &used.bound,
            &used.collected,
        );
        emitter.written.push(WrittenAccess::Closure {
            edges: used,
            from,
            seed,
            out,
        });
    }
    given_or_tested(emitter, other, out);
}
