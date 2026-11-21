//! The services of the `bodies` part: a body made ready (everything it reaches written, every site numbered), then written as
//! one record. A use of a definition is written the same whatever the caller is; its record is the pattern's form: a jump to
//! the pattern's program, kept by the cells of its bound columns, or a read of the definition's table by an index on them.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::constants::PROBE_MANY;
use crate::domain::code::filling::service::{stage_probe, stage_read, stage_use};
use crate::domain::code::filling::types::{ProbeWord, StageReadWord, UseWord};
use crate::domain::code::records::service::{
    call_entry_place, push_call_body, push_inline_body, push_table_body,
};
use crate::domain::code::records::types::{BoundFormal, CollectedRead};
use crate::domain::code::small_codes::service::{mode_code, source_code};
use crate::domain::code::types::{CodeEntry, MemoSite, PoolAt, WordSlot};
use crate::domain::code::types::{MeasuredForm, UnitKey};
use crate::domain::code::types::{Short, WordFlag};
use crate::domain::encoding::pipelines::types::StageUse;
use crate::domain::encoding::sites::service::{
    definition_table, fresh_register, push_operands, registers_list, short_of,
};
use crate::domain::encoding::sites::service::{fault, index, memo, pooled, reach_table, slot_of};
use crate::domain::encoding::types::PlacedWord;
use crate::domain::encoding::types::{
    DefinitionUse, Encoding, EntryPlace, PatternCode, PatternKey, ProgramRole, ReachedUnit,
    ReadyBody, UsedPattern,
};
use crate::domain::encoding::units::service::{begin_unit, end_unit};
use crate::domain::lowered::types::{DefinitionPattern, PackDefinition, PatternForm};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::MadeTableId;
use crate::domain::steps::types::StepProgram;
use crate::domain::steps::types::{
    ColumnMask, ReadColumn, Register, RegisterList, StepBody, StepOperand, StepRows,
};
use ryo_relations::domain::relations::types::Count;

/// The definition a use names and its pattern of the use's columns, with the pattern's place.
fn pattern_of<'p>(
    encoding: &Encoding<'p>,
    used: DefinitionUse<'_>,
) -> Result<UsedPattern<'p>, LoweringError> {
    let pack = encoding.pack;
    let at = usize::try_from(used.definition.0).map_err(|_| fault(encoding))?;
    let definition = pack.definitions.get(at).ok_or_else(|| fault(encoding))?;
    let place = definition
        .patterns
        .iter()
        .position(|pattern| pattern.bound == *used.bound && pattern.collected == *used.collected)
        .ok_or_else(|| fault(encoding))?;
    let pattern = definition
        .patterns
        .get(place)
        .ok_or_else(|| fault(encoding))?;
    let key = PatternKey {
        definition: used.definition,
        pattern: slot_of(encoding, Count(place), PastWidth::Sites)?,
    };
    Ok(UsedPattern {
        definition,
        pattern,
        key,
    })
}

/// A use of a definition made ready. A pattern in call form is reached, and written with the rule that first reached it; a
/// pattern in table form reaches its table. `outputs` are the registers a row's collected cells go to, in the collected
/// columns' order; without them the step that runs the body takes the cells itself.
pub(crate) fn ready_use(
    encoding: &mut Encoding<'_>,
    used: DefinitionUse<'_>,
    outputs: Option<&RegisterList>,
) -> Result<ReadyBody, LoweringError> {
    let UsedPattern {
        definition,
        pattern,
        key,
    } = pattern_of(encoding, used)?;
    if used.values.0.len() != used.bound.0.len() {
        return Err(fault(encoding));
    }
    let bound = used.bound.0.iter().zip(&used.values.0);
    match pattern.form {
        PatternForm::Call(_) => {
            let code = match encoding.patterns.get(&key) {
                Some(code) => *code,
                None => {
                    let code = PatternCode {
                        memo: memo(encoding)?,
                        entry: None,
                    };
                    let _new = encoding.patterns.insert(key, code);
                    encoding.reached.push(ReachedUnit::Pattern(key));
                    code
                }
            };
            let mut formals = Vec::with_capacity(used.bound.0.len());
            for (column, value) in bound {
                let formal = definition
                    .formals
                    .0
                    .get(usize::from(column.0))
                    .ok_or_else(|| fault(encoding))?;
                formals.push(BoundFormal {
                    formal: *formal,
                    value: pooled(encoding, value)?,
                });
            }
            Ok(ReadyBody::Call {
                pattern: key,
                code,
                formals,
            })
        }
        PatternForm::Table(table) => {
            reach_table(encoding, table)?;
            table_unit(encoding, definition, pattern, key)?;
            let columns: Vec<ReadColumn> = used
                .bound
                .0
                .iter()
                .map(|column| ReadColumn::Made(*column))
                .collect();
            let index = index(encoding, StepRows::Made(table), &columns, None)?;
            let values: Result<Vec<_>, LoweringError> =
                bound.map(|(_, value)| pooled(encoding, value)).collect();
            let reads = collected_reads(encoding, used.collected, outputs)?;
            Ok(ReadyBody::Table {
                index,
                values: values?,
                reads,
            })
        }
    }
}

/// The reads of a row's collected columns: each into the register of its place among `outputs`, or into none where the step
/// that runs the body takes the cell itself.
fn collected_reads(
    encoding: &Encoding<'_>,
    collected: &ColumnMask,
    outputs: Option<&RegisterList>,
) -> Result<Vec<CollectedRead>, LoweringError> {
    let mut reads = Vec::with_capacity(collected.0.len());
    for (place, column) in collected.0.iter().enumerate() {
        let out = match outputs {
            Some(outputs) => *outputs.0.get(place).ok_or_else(|| fault(encoding))?,
            None => Register(NO_SLOT.0),
        };
        reads.push(CollectedRead {
            column: *column,
            out,
        });
    }
    Ok(reads)
}

/// The unit of a pattern in table form, once: it has no word of its own (its table's fill is the table's unit), and the
/// statistics measure it under its own name.
fn table_unit(
    encoding: &mut Encoding<'_>,
    definition: &PackDefinition,
    pattern: &DefinitionPattern,
    key: PatternKey,
) -> Result<(), LoweringError> {
    if encoding.patterns.contains_key(&key) {
        return Ok(());
    }
    let _new = encoding.patterns.insert(
        key,
        PatternCode {
            memo: MemoSite(NO_SLOT.0),
            entry: None,
        },
    );
    // The unit being written goes on after this one: a table-form pattern's unit holds no plan point.
    let writing = encoding.unit;
    let name = UnitKey::Pattern(
        definition.node,
        pattern.bound.clone(),
        pattern.collected.clone(),
    );
    begin_unit(encoding, name, MeasuredForm::Table, pattern.shape)?;
    end_unit(encoding)?;
    encoding.unit = writing;
    Ok(())
}

/// A body made ready: an inline body's steps written by `written` (the programs' own writer, handed in so that a body and the
/// program that holds it do not name each other), a use of a definition numbered.
pub(crate) fn ready_body(
    encoding: &mut Encoding<'_>,
    body: &StepBody,
    outputs: Option<&RegisterList>,
    written: fn(&mut Encoding<'_>, &StepProgram, ProgramRole) -> Result<CodeEntry, LoweringError>,
) -> Result<ReadyBody, LoweringError> {
    match body {
        StepBody::Inline(program) => Ok(ReadyBody::Inline(written(
            encoding,
            program,
            ProgramRole::Keyed,
        )?)),
        StepBody::Definition {
            definition,
            bound,
            collected,
            values,
        } => {
            let used = DefinitionUse {
                definition: *definition,
                bound,
                collected,
                values,
            };
            ready_use(encoding, used, outputs)
        }
    }
}

/// Writes a ready body as one record, answering where it starts. The entry of a pattern not yet written is written again at
/// the end of the encoding.
pub(crate) fn push_body(encoding: &mut Encoding<'_>, body: &ReadyBody) -> PoolAt {
    match body {
        ReadyBody::Inline(entry) => push_inline_body(&mut encoding.code.pool, *entry),
        ReadyBody::Call {
            pattern,
            code,
            formals,
        } => {
            let at = push_call_body(
                &mut encoding.code.pool,
                code.entry.unwrap_or(CodeEntry(0)),
                code.memo,
                formals,
            );
            encoding.entries.push(EntryPlace {
                place: call_entry_place(at),
                pattern: *pattern,
            });
            at
        }
        ReadyBody::Table {
            index,
            values,
            reads,
        } => push_table_body(&mut encoding.code.pool, *index, values, reads),
    }
}

/// Writes every waiting entry: each pattern reached has its program by now.
pub(crate) fn entries_written(encoding: &mut Encoding<'_>) -> Result<(), LoweringError> {
    for waiting in std::mem::take(&mut encoding.entries) {
        let entry = encoding
            .patterns
            .get(&waiting.pattern)
            .and_then(|code| code.entry);
        let at = usize::try_from(waiting.place.0).unwrap_or(usize::MAX);
        match (entry, encoding.code.pool.get_mut(at)) {
            (Some(entry), Some(place)) => *place = WordSlot(entry.0),
            (Some(_) | None, None) | (None, Some(_)) => return Err(fault(encoding)),
        }
    }
    Ok(())
}

/// Whether an operand is a register: what a stage's one-value form holds.
#[must_use]
pub(crate) const fn register_of(operand: &StepOperand) -> Option<Register> {
    match operand {
        StepOperand::Register(register) => Some(*register),
        StepOperand::Word(_) | StepOperand::Words(_) | StepOperand::File => None,
    }
}

/// The words of a `Use`: a pattern in call form is one word over its body record; a pattern in table form is a probe of the
/// definition's table by the bound columns, then one read for each collected column, through a row handle no stage names.
pub(crate) fn use_words(
    encoding: &mut Encoding<'_>,
    used: StageUse<'_>,
    place: WordSlot,
    words: &mut Vec<PlacedWord>,
) -> Result<(), LoweringError> {
    let named = DefinitionUse {
        definition: used.definition,
        bound: used.bound,
        collected: used.collected,
        values: used.values,
    };
    let ready = ready_use(encoding, named, Some(used.outputs))?;
    let mode = WordFlag(mode_code(used.mode).0);
    match &ready {
        ReadyBody::Call { code, .. } => {
            let memo = code.memo;
            let body = push_body(encoding, &ready);
            let registers = registers_list(encoding, used.outputs);
            let outputs = short_of(
                encoding,
                Count(used.outputs.0.len()),
                PastWidth::WordOperands,
            )?;
            words.push(PlacedWord {
                word: stage_use(UseWord {
                    flag: mode,
                    outputs,
                    body,
                    registers,
                    memo,
                }),
                point: place,
            });
        }
        ReadyBody::Table {
            index: site,
            values,
            reads,
        } => {
            let rows = match used.mode {
                UseMode::Rows => true,
                UseMode::Present | UseMode::Absent => false,
            };
            // Only a use whose rows go on reads a column of a row found.
            if !rows && !reads.is_empty() {
                return Err(fault(encoding));
            }
            let handle = fresh_register(encoding)?;
            let at = push_operands(encoding, values);
            let probe = ProbeWord {
                flag: WordFlag(mode.0 | PROBE_MANY.0),
                values: short_of(encoding, Count(values.len()), PastWidth::WordOperands)?,
                out: handle,
                probed: WordSlot(at.0),
                index: *site,
            };
            words.push(PlacedWord {
                word: stage_probe(probe),
                point: place,
            });
            let table = table_of(encoding, used)?;
            for read in reads {
                let word = stage_read(StageReadWord {
                    flag: source_code(StepRows::Made(table)),
                    column: Short(read.column.0),
                    row: handle,
                    out: read.out,
                    read: WordSlot(table.0),
                });
                words.push(PlacedWord {
                    word,
                    point: NO_SLOT,
                });
            }
        }
        ReadyBody::Inline(_) => return Err(fault(encoding)),
    }
    Ok(())
}

/// The table a use in table form reads: its definition's.
fn table_of(encoding: &mut Encoding<'_>, used: StageUse<'_>) -> Result<MadeTableId, LoweringError> {
    definition_table(encoding, used.definition)
}
