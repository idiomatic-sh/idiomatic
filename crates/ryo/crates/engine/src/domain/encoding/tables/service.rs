//! The services of the `tables` part: a made table's fill written as its pipelines and its record, and every unit a rule
//! reached written after the rule, in the order reached, so a count past its width is the refusal of the rule that reached it.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::next_place;
use crate::domain::code::types::{MeasuredForm, ShapeChoice, UnitKey};
use crate::domain::code::types::{Short, TableCode, WordSlot};
use crate::domain::encoding::needs::service::{definition_need, need_site};
use crate::domain::encoding::pipelines::service::pipeline_entry;
use crate::domain::encoding::programs::service::program_entry;
use crate::domain::encoding::sites::service::{constant, fault, index, past, slot_of};
use crate::domain::encoding::types::{Encoding, ReachedUnit};
use crate::domain::encoding::types::{PatternKey, ProgramRole};
use crate::domain::encoding::units::service::{begin_unit, end_unit};
use crate::domain::lowered::service::{fill_code, match_left_width};
use crate::domain::lowered::types::PatternForm;
use crate::domain::lowered::types::{
    FillInput, GraphFill, GroupFill, MadeFill, MadeTable, OrderedFill,
};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::stages::types::Pipeline;
use crate::domain::steps::types::{MadeTableId, ReadColumn, StepRows};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types::RepeatedRows;

/// A list as a record holds it in place: its length, then its items.
fn listed(
    encoding: &Encoding<'_>,
    record: &mut Vec<WordSlot>,
    items: &[WordSlot],
) -> Result<(), LoweringError> {
    record.push(slot_of(encoding, Count(items.len()), PastWidth::Sites)?);
    record.extend_from_slice(items);
    Ok(())
}

/// Writes the pipelines of one input of a fill and adds the input to the fill's record: the list of its pipelines' entries,
/// the list of its key columns, and its order column or none.
fn input_written(
    encoding: &mut Encoding<'_>,
    table: MadeTableId,
    record: &mut Vec<WordSlot>,
    pipelines: &[Pipeline],
    keys: &[WordSlot],
    order: WordSlot,
    repeats: RepeatedRows,
) -> Result<(), LoweringError> {
    let entries: Result<Vec<WordSlot>, LoweringError> = pipelines
        .iter()
        .map(|pipeline| {
            pipeline_entry(encoding, table, pipeline, repeats).map(|entry| WordSlot(entry.0))
        })
        .collect();
    listed(encoding, record, &entries?)?;
    listed(encoding, record, keys)?;
    record.push(order);
    Ok(())
}

/// Adds an input of a group, ordered or graph fill to the fill's record.
fn fill_input(
    encoding: &mut Encoding<'_>,
    table: MadeTableId,
    record: &mut Vec<WordSlot>,
    input: &FillInput,
) -> Result<(), LoweringError> {
    let keys: Vec<WordSlot> = input
        .keys
        .0
        .iter()
        .map(|column| WordSlot(u32::from(column.0)))
        .collect();
    let order = input
        .order
        .map_or(NO_SLOT, |column| WordSlot(u32::from(column.0)));
    input_written(
        encoding,
        table,
        record,
        &input.pipelines,
        &keys,
        order,
        RepeatedRows::Kept,
    )
}

/// The one parameter of a fill's record: the distinct column of a count (or none), the column a digest is over, the end a
/// group keeps (0 the least, 1 the greatest), whether a definition's rows drop repeats, how many rows a constant table has.
fn parameter(encoding: &Encoding<'_>, fill: &MadeFill) -> Result<WordSlot, LoweringError> {
    Ok(match fill {
        MadeFill::Rows(_, repeats) => match repeats {
            RepeatedRows::Dropped => WordSlot(1),
            RepeatedRows::Kept => WordSlot(0),
        },
        MadeFill::Group(group, _) => match group {
            GroupFill::Count { distinct } => {
                distinct.map_or(NO_SLOT, |column| WordSlot(u32::from(column.0)))
            }
            GroupFill::Digest { over } => WordSlot(u32::from(over.0)),
            GroupFill::Ends(end) => match end {
                GroupEnd::Least => WordSlot(0),
                GroupEnd::Greatest => WordSlot(1),
            },
            GroupFill::Places => WordSlot(0),
        },
        MadeFill::Ordered(ordered) => match ordered {
            OrderedFill::Sorted(_) | OrderedFill::Match(..) => WordSlot(0),
        },
        MadeFill::Graph(graph, _) => match graph {
            GraphFill::FixedPoints
            | GraphFill::OrderedDfs
            | GraphFill::Worklist
            | GraphFill::Cycles => WordSlot(0),
        },
        MadeFill::Constant(rows) => slot_of(encoding, Count(rows.0.len()), PastWidth::Sites)?,
    })
}

/// The rest of a fill's record after its parameter: each input with its pipelines written, or a constant table's rows as
/// constants, row by row.
fn inputs_written(
    encoding: &mut Encoding<'_>,
    table: MadeTableId,
    fill: &MadeFill,
    record: &mut Vec<WordSlot>,
) -> Result<(), LoweringError> {
    match fill {
        MadeFill::Rows(pipelines, repeats) => {
            input_written(encoding, table, record, pipelines, &[], NO_SLOT, *repeats)
        }
        MadeFill::Group(_, input) => fill_input(encoding, table, record, input),
        MadeFill::Ordered(ordered) => match ordered {
            OrderedFill::Sorted(input) => fill_input(encoding, table, record, input),
            OrderedFill::Match(left, right) => {
                fill_input(encoding, table, record, left)?;
                fill_input(encoding, table, record, right)
            }
        },
        MadeFill::Graph(_, inputs) => inputs
            .iter()
            .try_for_each(|input| fill_input(encoding, table, record, input)),
        MadeFill::Constant(rows) => {
            for cell in rows.0.iter().flat_map(|row| row.iter()) {
                record.push(WordSlot(constant(encoding, *cell)?.0));
            }
            Ok(())
        }
    }
}

/// Writes a reached table as its own unit: its pipelines, then its record (its width, the list of its index sites, its one
/// parameter, then each input), and the need it waits on.
fn table_written(encoding: &mut Encoding<'_>, id: MadeTableId) -> Result<(), LoweringError> {
    let pack = encoding.pack;
    let place = usize::try_from(id.0).map_err(|_| fault(encoding))?;
    let table: &MadeTable = pack.tables.get(place).ok_or_else(|| fault(encoding))?;
    let code = fill_code(&table.fill);
    // An ordered match's row is its two inputs' rows, one after the other: a table of any other width is not one.
    if let MadeFill::Ordered(OrderedFill::Match(left, right)) = &table.fill
        && match_left_width(left, right, table.width).is_none()
    {
        return Err(fault(encoding));
    }
    begin_unit(
        encoding,
        UnitKey::Table(table.node, code),
        MeasuredForm::Call,
        ShapeChoice::Decided,
    )?;
    let unit = encoding.unit.unit;
    let width =
        u16::try_from(table.width.0).map_err(|_| past(encoding, PastWidth::TableColumns))?;
    let mut sites = Vec::with_capacity(table.indexes.len());
    for columns in &table.indexes {
        let columns: Vec<ReadColumn> = columns
            .0
            .iter()
            .map(|column| ReadColumn::Made(*column))
            .collect();
        sites.push(WordSlot(
            index(encoding, StepRows::Made(id), &columns, None)?.0,
        ));
    }
    let mut record = vec![WordSlot(u32::from(width))];
    listed(encoding, &mut record, &sites)?;
    record.push(parameter(encoding, &table.fill)?);
    inputs_written(encoding, id, &table.fill, &mut record)?;
    end_unit(encoding)?;
    let need = need_site(encoding, &table.emptiness)?;
    let operands = WordSlot(next_place(&encoding.code.pool).0);
    encoding.code.pool.extend(record);
    match encoding.code.tables.get_mut(place) {
        Some(written) => {
            *written = TableCode {
                fill: Short(code.0),
                operands,
                need,
                unit,
            };
            Ok(())
        }
        None => Err(fault(encoding)),
    }
}

/// Writes the program of a reached pattern in call form, as its own unit.
fn pattern_written(encoding: &mut Encoding<'_>, key: PatternKey) -> Result<(), LoweringError> {
    let pack = encoding.pack;
    let place = |number: WordSlot| usize::try_from(number.0).unwrap_or(usize::MAX);
    let definition = pack.definitions.get(place(WordSlot(key.definition.0)));
    let pattern = definition.and_then(|definition| {
        definition
            .patterns
            .get(place(key.pattern))
            .map(|pattern| (definition, pattern))
    });
    let Some((definition, pattern)) = pattern else {
        return Err(fault(encoding));
    };
    let PatternForm::Call(program) = &pattern.form else {
        return Err(fault(encoding));
    };
    let name = UnitKey::Pattern(
        definition.node,
        pattern.bound.clone(),
        pattern.collected.clone(),
    );
    begin_unit(encoding, name, MeasuredForm::Call, pattern.shape)?;
    let unit = encoding.unit.unit;
    let entry = program_entry(encoding, program, ProgramRole::Keyed)?;
    end_unit(encoding)?;
    match encoding.patterns.get_mut(&key) {
        Some(code) => {
            (code.entry, code.unit) = (Some(entry), Some(unit));
            Ok(())
        }
        None => Err(fault(encoding)),
    }
}

/// Writes every unit reached and not yet written, in the order reached: each may reach more.
pub(crate) fn reached_written(encoding: &mut Encoding<'_>) -> Result<(), LoweringError> {
    while let Some(unit) = encoding.reached.get(encoding.written.0).copied() {
        encoding.written = Count(encoding.written.0.saturating_add(1));
        match unit {
            ReachedUnit::Pattern(pattern) => pattern_written(encoding, pattern)?,
            ReachedUnit::Table(table) => table_written(encoding, table)?,
            ReachedUnit::Need(definition, site) => definition_need(encoding, definition, site)?,
        }
    }
    Ok(())
}
