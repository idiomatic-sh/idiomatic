//! The services of the `closures` part: a closure's walk as an access. A program runs it in place: from the source over the
//! edges, or from what is reached back over them, the edges' definition asked by the element in hand. A pipeline holds no
//! walk: it uses the pattern of its own definition that binds the keys and the seed and collects the other end, whose
//! program is this walk, run once per distinct seed.

use crate::domain::lowering::costing::types::CellState;
use crate::domain::lowering::definitions::service::{asked, fresh_register};
use crate::domain::lowering::flattening::service::made_column;
use crate::domain::lowering::flattening::types::{
    EdgeColumns, EdgeRows, FlatAtom, FlatTerm, RowColumn,
};
use crate::domain::lowering::planning::closures::types::{AtomWriter, BoundCell, EdgeEnds};
use crate::domain::lowering::planning::operands::service::{
    given_or_tested, operand, register_of, state_of,
};
use crate::domain::lowering::planning::types::{EdgeUse, WalkEdges};
use crate::domain::lowering::planning::types::{Emitter, Enumerated, UnitRole, WrittenAccess};
use crate::domain::lowering::planning::walks::types::WalkEnds;
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
/// the closure both call it. It answers the register the other end goes to; none, with the miss counted, when a term is no
/// column of the unit's row.
pub(crate) fn own_used(
    emitter: &mut Emitter<'_>,
    unit: DefinitionId,
    given: &[FlatTerm],
    other: FlatTerm,
    out: Register,
) -> Option<Register> {
    let Some(used) = own_use(emitter, unit, given, other) else {
        emitter.missed = Count(emitter.missed.0.saturating_add(1));
        return None;
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
    Some(out)
}

/// The edges of a step: a use of its definition bound on the element in `from` and on the keys.
fn defined_edges(
    emitter: &mut Emitter<'_>,
    definition: DefinitionId,
    edges: &EdgeColumns,
    keys: &[FlatTerm],
    asks: EdgeEnds,
    from: Register,
) -> Option<WalkEdges> {
    let mut cells = vec![BoundCell {
        column: asks.asked,
        value: StepOperand::Register(from),
    }];
    for (column, key) in edges.keys.iter().zip(keys) {
        cells.push(BoundCell {
            column: *column,
            value: operand(&emitter.environment, *key)?,
        });
    }
    let used = edge_use(definition, cells, asks.gives);
    asked(
        emitter.drafts,
        used.definition,
        &used.bound,
        &used.collected,
    );
    Some(WalkEdges::Use(used))
}

/// The edges of a source's own rows: the read of one row of it, written in place by the emitter's own writer, with the
/// asked column's cell in `from` and each key's cell in the register its key is in. None when a key is in no register.
fn read_edges(
    emitter: &mut Emitter<'_>,
    edges: &EdgeColumns,
    keys: &[FlatTerm],
    asks: EdgeEnds,
    from: Register,
    write: AtomWriter,
) -> Option<WalkEdges> {
    let EdgeRows::Read {
        rows,
        cells,
        per_file,
    } = &edges.rows
    else {
        return None;
    };
    let cell = |column: MadeColumn| match cells.get(usize::from(column.0)) {
        Some(FlatTerm::Variable(variable)) => Some(*variable),
        Some(FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File) | None => None,
    };
    let (asked_cell, given_cell) = (cell(asks.asked)?, cell(asks.gives)?);
    let _new = emitter.environment.registers.insert(asked_cell, from);
    for (column, key) in edges.keys.iter().zip(keys) {
        let StepOperand::Register(register) = operand(&emitter.environment, *key)? else {
            return None;
        };
        let _new = emitter
            .environment
            .registers
            .insert(cell(*column)?, register);
    }
    // The walk reads the cell it gives; no other cell of the row is read by anything.
    let _new = emitter.uses.insert(given_cell, Count(2));
    let outer = std::mem::take(&mut emitter.written);
    write(
        emitter,
        &FlatAtom::Rows {
            rows: rows.clone(),
            cells: cells.clone(),
            per_file: *per_file,
        },
    );
    let steps = std::mem::replace(&mut emitter.written, outer);
    let gives = *emitter.environment.registers.get(&given_cell)?;
    Some(WalkEdges::Inline { steps, gives })
}

/// Writes a closure's walk. Entered from its source it gives what is reached; entered from what is reached it gives the
/// sources, over the edges read from their destination. An end that was given already is tested against the walk's.
pub(crate) fn closed(
    emitter: &mut Emitter<'_>,
    edges: &EdgeColumns,
    keys: &[FlatTerm],
    ends: WalkEnds,
    write: AtomWriter,
) {
    let forward = state_of(&emitter.environment, ends.start) == CellState::Register;
    let (seed_term, other) = if forward {
        (ends.start, ends.reached)
    } else {
        (ends.reached, ends.start)
    };
    let asks = if forward {
        EdgeEnds {
            asked: edges.source,
            gives: edges.destination,
        }
    } else {
        EdgeEnds {
            asked: edges.destination,
            gives: edges.source,
        }
    };
    let Some(seed) = register_of(emitter, seed_term) else {
        emitter.missed = Count(emitter.missed.0.saturating_add(1));
        return;
    };
    emitter.environment.enumerated = Enumerated::Already;
    let out = fresh_register(emitter.drafts);
    if let (UnitRole::Pipeline, Some(unit)) = (emitter.role, emitter.unit) {
        let given: Vec<FlatTerm> = keys.iter().copied().chain([seed_term]).collect();
        if own_used(emitter, unit, &given, other, out).is_none() {
            return;
        }
    } else {
        // The element whose edges are asked is written into `from` before the edges run.
        let from: Register = fresh_register(emitter.drafts);
        let walked = match &edges.rows {
            EdgeRows::Definition(definition) => {
                defined_edges(emitter, *definition, edges, keys, asks, from)
            }
            EdgeRows::Read { .. } => read_edges(emitter, edges, keys, asks, from, write),
        };
        let Some(walked) = walked else {
            emitter.missed = Count(emitter.missed.0.saturating_add(1));
            return;
        };
        emitter.written.push(WrittenAccess::Closure {
            edges: walked,
            from,
            seed,
            out,
        });
    }
    given_or_tested(emitter, other, out);
}
