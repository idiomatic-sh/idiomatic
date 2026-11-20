//! The services of the `sites` part: every number the encoding gives out, and every count it writes at a width. A count that
//! does not fit its width is the refusal of the rule being written, naming what was counted: nothing is ever cut to fit.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::{next_place, push_list, push_operand};
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::small_codes::service::node_column_code;
use crate::domain::code::types::{
    ConstantNumber, IndexCode, IndexSite, MemoSite, PoolAt, RelationNumber, Short, WordSlot,
};
use crate::domain::encoding::constants::LAST_STEP;
use crate::domain::encoding::types::{
    Encoding, IndexedBy, NumberedSource, ReachedUnit, SlotValue, TableProgress,
};
use crate::domain::lowered::types::{DefinitionPattern, PatternForm};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{LoadRefusal, PastWidth};
use crate::domain::steps::types::{
    DefinitionId, MadeTableId, ReadColumn, Register, RegisterCount, RegisterList, StepOperand,
    StepOperands, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Count, Relation};
use ryo_values::domain::plan_parts::service::column_place;

/// The refusal of the rule being written for a count past its width.
pub(crate) const fn past(encoding: &Encoding<'_>, width: PastWidth) -> LoweringError {
    LoweringError::Refused {
        rule: encoding.rule,
        refusal: LoadRefusal::Budget(width),
    }
}

/// The encoding's own fault, of the rule being written: a step or a stage the contract does not let stand where it stands, or
/// names what the pack does not hold. The lowering proves a pack before it answers it, so this is the engine's fault.
pub(crate) const fn fault(encoding: &Encoding<'_>) -> LoweringError {
    LoweringError::Refused {
        rule: encoding.rule,
        refusal: LoadRefusal::Unlowerable(LAST_STEP),
    }
}

/// A count as a word's short operand.
pub(crate) fn short_of(
    encoding: &Encoding<'_>,
    count: Count,
    width: PastWidth,
) -> Result<Short, LoweringError> {
    u16::try_from(count.0)
        .map(Short)
        .map_err(|_| past(encoding, width))
}

/// A count as a slot.
pub(crate) fn slot_of(
    encoding: &Encoding<'_>,
    count: Count,
    width: PastWidth,
) -> Result<WordSlot, LoweringError> {
    u32::try_from(count.0)
        .map(WordSlot)
        .map_err(|_| past(encoding, width))
}

/// The number of a constant: one number for each value.
pub(crate) fn constant(
    encoding: &mut Encoding<'_>,
    word: CellWord,
) -> Result<ConstantNumber, LoweringError> {
    if let Some(number) = encoding.constants.get(&word) {
        return Ok(*number);
    }
    let number = ConstantNumber(
        slot_of(
            encoding,
            Count(encoding.code.constants.len()),
            PastWidth::Sites,
        )?
        .0,
    );
    encoding.code.constants.push(word);
    let _new = encoding.constants.insert(word, number);
    Ok(number)
}

/// The number of a relation among the relations the code reads.
pub(crate) fn relation(
    encoding: &mut Encoding<'_>,
    read: Relation,
) -> Result<RelationNumber, LoweringError> {
    let place = match encoding
        .code
        .relations
        .iter()
        .position(|each| *each == read)
    {
        Some(place) => place,
        None => {
            encoding.code.relations.push(read);
            encoding.code.relations.len().saturating_sub(1)
        }
    };
    Ok(RelationNumber(
        slot_of(encoding, Count(place), PastWidth::Sites)?.0,
    ))
}

/// A memo site.
pub(crate) fn memo(encoding: &mut Encoding<'_>) -> Result<MemoSite, LoweringError> {
    let site = MemoSite(encoding.memos.0);
    encoding.memos = WordSlot(
        encoding
            .memos
            .0
            .checked_add(1)
            .ok_or_else(|| past(encoding, PastWidth::Sites))?,
    );
    Ok(site)
}

/// A register no step or stage names: the row handle between a probe of a definition's table and the reads of its columns.
pub(crate) fn fresh_register(encoding: &mut Encoding<'_>) -> Result<Register, LoweringError> {
    let register = Register(encoding.registers.0);
    let next = register
        .0
        .checked_add(1)
        .ok_or_else(|| past(encoding, PastWidth::Registers))?;
    encoding.registers = RegisterCount(next);
    Ok(register)
}

/// A column as its source's column code: a node column's code, a filled column's place in its relation, a made column's place.
pub(crate) fn column_code(
    encoding: &Encoding<'_>,
    column: ReadColumn,
) -> Result<Short, LoweringError> {
    match column {
        ReadColumn::Node(node) => Ok(node_column_code(node)),
        ReadColumn::Filled(filled) => u16::try_from(column_place(filled).0)
            .map(Short)
            .map_err(|_| past(encoding, PastWidth::TableColumns)),
        ReadColumn::Made(made) => Ok(Short(made.0)),
    }
}

/// The table a definition's rows are read from with nothing bound: the table of its pattern that binds nothing, or of any of
/// its patterns in table form. Reading it reaches it.
pub(crate) fn definition_table(
    encoding: &mut Encoding<'_>,
    definition: DefinitionId,
) -> Result<MadeTableId, LoweringError> {
    let (pack, place) = (
        encoding.pack,
        usize::try_from(definition.0).map_err(|_| fault(encoding))?,
    );
    let patterns = &pack
        .definitions
        .get(place)
        .ok_or_else(|| fault(encoding))?
        .patterns;
    let own = |pattern: &DefinitionPattern| match pattern.form {
        PatternForm::Table(table) => Some(table),
        PatternForm::Call(_) => None,
    };
    let unbound = patterns
        .iter()
        .filter(|pattern| pattern.bound.0.is_empty())
        .find_map(own);
    let table = unbound
        .or_else(|| patterns.iter().find_map(own))
        .ok_or_else(|| fault(encoding))?;
    reach_table(encoding, table)?;
    Ok(table)
}

/// Marks a made table read: its fill is written with the rule that first reached it.
pub(crate) fn reach_table(
    encoding: &mut Encoding<'_>,
    table: MadeTableId,
) -> Result<(), LoweringError> {
    let place = usize::try_from(table.0).map_err(|_| fault(encoding))?;
    let Some(progress) = encoding.tables.get_mut(place) else {
        return Err(fault(encoding));
    };
    if *progress == TableProgress::Unreached {
        *progress = TableProgress::Reached;
        encoding.reached.push(ReachedUnit::Table(table));
    }
    Ok(())
}

/// Where a source's rows are numbered: 0 the node table, 1 a filled relation, 2 a made table; and the relation's or the
/// table's number. Naming a table reaches it.
pub(crate) fn source_number(
    encoding: &mut Encoding<'_>,
    source: StepRows,
) -> Result<NumberedSource, LoweringError> {
    let (source, number) = match source {
        StepRows::Node(read) => (0, relation(encoding, read)?.0),
        StepRows::Filled(read) => (1, relation(encoding, read)?.0),
        StepRows::Made(table) | StepRows::Written(table) => {
            reach_table(encoding, table)?;
            (2, table.0)
        }
        StepRows::Definition(definition) => (2, definition_table(encoding, definition)?.0),
    };
    Ok(NumberedSource {
        source: WordSlot(source),
        number: WordSlot(number),
    })
}

/// The site of the index of a source by some of its columns, sorted within a key by `ordered` when one is given: one site for
/// one index, whoever asks.
pub(crate) fn index(
    encoding: &mut Encoding<'_>,
    source: StepRows,
    columns: &[ReadColumn],
    ordered: Option<ReadColumn>,
) -> Result<IndexSite, LoweringError> {
    let rows = source_number(encoding, source)?;
    let mut by = IndexedBy {
        rows,
        key: Vec::with_capacity(columns.len()),
        sorted: NO_SLOT,
    };
    for column in columns {
        by.key
            .push(WordSlot(u32::from(column_code(encoding, *column)?.0)));
    }
    if let Some(ordered) = ordered {
        by.sorted = WordSlot(u32::from(column_code(encoding, ordered)?.0));
    }
    if let Some(site) = encoding.indexes.get(&by) {
        return Ok(*site);
    }
    let site = IndexSite(
        slot_of(
            encoding,
            Count(encoding.code.indexes.len()),
            PastWidth::Sites,
        )?
        .0,
    );
    let columns = push_list(&mut encoding.code.pool, &by.key);
    encoding.code.indexes.push(IndexCode {
        source: Short(u16::try_from(rows.source.0).map_err(|_| fault(encoding))?),
        number: rows.number,
        columns: WordSlot(columns.0),
        ordered: by.sorted,
    });
    let _new = encoding.indexes.insert(by, site);
    Ok(site)
}

/// An operand as the pool holds it; a set of constants is written to the pool as a sorted list first.
pub(crate) fn pooled(
    encoding: &mut Encoding<'_>,
    operand: &StepOperand,
) -> Result<PooledOperand, LoweringError> {
    Ok(match operand {
        StepOperand::Register(register) => PooledOperand::Register(*register),
        StepOperand::Word(word) => PooledOperand::Constant(constant(encoding, *word)?),
        StepOperand::Words(words) => PooledOperand::Constants(constants_list(encoding, words)?),
        StepOperand::File => PooledOperand::File,
    })
}

/// StepOperands as the pool holds them, with what each reaches written.
pub(crate) fn pooled_each(
    encoding: &mut Encoding<'_>,
    operands: &[StepOperand],
) -> Result<Vec<PooledOperand>, LoweringError> {
    operands
        .iter()
        .map(|operand| pooled(encoding, operand))
        .collect()
}

/// Writes operands one after the other, answering where they start.
pub(crate) fn push_operands(encoding: &mut Encoding<'_>, operands: &[PooledOperand]) -> PoolAt {
    let at = next_place(&encoding.code.pool);
    for operand in operands {
        push_operand(&mut encoding.code.pool, *operand);
    }
    at
}

/// Writes a step's operands one after the other, answering where they start.
pub(crate) fn operands_place(
    encoding: &mut Encoding<'_>,
    operands: &StepOperands,
) -> Result<PoolAt, LoweringError> {
    let ready = pooled_each(encoding, &operands.0)?;
    Ok(push_operands(encoding, &ready))
}

/// Writes a set of constants as a list of their numbers, sorted by value.
pub(crate) fn constants_list(
    encoding: &mut Encoding<'_>,
    words: &[CellWord],
) -> Result<PoolAt, LoweringError> {
    let mut sorted = words.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let numbers: Result<Vec<WordSlot>, LoweringError> = sorted
        .into_iter()
        .map(|word| constant(encoding, word).map(|number| WordSlot(number.0)))
        .collect();
    Ok(push_list(&mut encoding.code.pool, &numbers?))
}

/// Writes registers as a list.
pub(crate) fn registers_list(encoding: &mut Encoding<'_>, registers: &RegisterList) -> PoolAt {
    let slots: Vec<WordSlot> = registers
        .0
        .iter()
        .map(|register| WordSlot(register.0))
        .collect();
    push_list(&mut encoding.code.pool, &slots)
}

/// An operand a word holds in one slot: a register or a constant's number; none for a set and for the file, which one slot
/// does not hold.
pub(crate) fn slot_value(
    encoding: &mut Encoding<'_>,
    operand: &StepOperand,
) -> Result<Option<SlotValue>, LoweringError> {
    Ok(match operand {
        StepOperand::Register(register) => Some(SlotValue::Register(WordSlot(register.0))),
        StepOperand::Word(word) => {
            Some(SlotValue::Constant(WordSlot(constant(encoding, *word)?.0)))
        }
        StepOperand::Words(_) | StepOperand::File => None,
    })
}
