//! The services of the `pipelines` part: a pipeline written as its stages' words and the write of its row. It holds where a
//! file-sized stage may stand: a `Scan` first, and a `Product`; any other, and a `Scan` that is not first, is the encoding's
//! fault of the rule being written.

use crate::domain::code::constants::{
    BUCKET, NO_SLOT, PROBE_HIGH_INCLUSIVE, PROBE_LOW_INCLUSIVE, PROBE_MANY, PROBE_RANGE,
    REPEATS_DROPPED, WALK_AXIS,
};
use crate::domain::code::filling::service::{
    stage_call, stage_keep, stage_latest, stage_probe, stage_product, stage_read, stage_render,
    stage_scan, stage_walk, stage_write,
};
use crate::domain::code::filling::types::{
    ProbeWord, ProductWord, ScanWord, StageReadWord, WalkWord, WriteWord,
};
use crate::domain::code::records::service::{push_list, push_operand};
use crate::domain::code::small_codes::service::{direction_code, mode_code, source_code};
use crate::domain::code::types::{CodeEntry, CodeWord, PoolAt, WordFlag, WordSlot};
use crate::domain::encoding::bodies::service::register_of;
use crate::domain::encoding::bodies::service::use_words;
use crate::domain::encoding::comparisons::service::{
    call_operands, latest_operands, render_operands, test_operands,
};
use crate::domain::encoding::matches::service::{extent_flag, size_site};
use crate::domain::encoding::pipelines::types::{PipelinePlace, ProbeRead, StageUse};
use crate::domain::encoding::programs::types::NearestRows;
use crate::domain::encoding::sites::service::{
    column_code, constant, fault, index, operands_place, pooled, pooled_each, push_operands,
    short_of, source_number,
};
use crate::domain::encoding::types::{Encoding, PlacedWord};
use crate::domain::encoding::units::service::{placed, point};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::service::stage_class;
use crate::domain::steps::stages::types::{FillStage, Pipeline, TreeWalk};
use crate::domain::steps::types::{
    BoundKind, CostClass, MadeTableId, MatchColumns, ReadColumn, Register, StepOperand,
    StepOperands, StepRows,
};
use ryo_language::domain::node_table::types::FieldCode;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::RepeatedRows;

/// The field of a walk of every child: field 0 is no field.
const NO_FIELD: FieldCode = FieldCode(0);

/// Holds a stage to its place: a file-sized stage is a `Scan` standing first, or a `Product`; any other is the encoding's
/// fault of the rule being written.
fn held(
    encoding: &Encoding<'_>,
    stage: &FillStage,
    place: PipelinePlace,
) -> Result<(), LoweringError> {
    match stage_class(stage) {
        CostClass::Constant | CostClass::Fanout | CostClass::Group => Ok(()),
        CostClass::File => match (stage, place) {
            (FillStage::Scan { .. }, PipelinePlace::First)
            | (FillStage::Product { .. }, PipelinePlace::First | PipelinePlace::Later) => Ok(()),
            (FillStage::Scan { .. }, PipelinePlace::Later)
            | (
                FillStage::Probe { .. }
                | FillStage::Use { .. }
                | FillStage::Latest { .. }
                | FillStage::Read { .. }
                | FillStage::Keep { .. }
                | FillStage::Call { .. }
                | FillStage::Render { .. }
                | FillStage::Walk { .. },
                PipelinePlace::First | PipelinePlace::Later,
            ) => Err(fault(encoding)),
        },
    }
}

/// The word of a `Scan`: every row of a source, or of one bucket of it that constants name.
fn scan_word(
    encoding: &mut Encoding<'_>,
    source: StepRows,
    columns: &MatchColumns,
    values: &StepOperands,
    out: Register,
) -> Result<CodeWord, LoweringError> {
    let flag = source_code(source).0 | extent_flag(source).0;
    if columns.0.is_empty() && values.0.is_empty() {
        let read = source_number(encoding, source)?.number;
        return Ok(stage_scan(ScanWord {
            flag: WordFlag(flag),
            out,
            constants: PoolAt(0),
            read,
        }));
    }
    // A bucket is named by one constant for each of its columns: a scan has no row to read a register of.
    let mut bucket = Vec::with_capacity(values.0.len());
    for value in &values.0 {
        match value {
            StepOperand::Word(word) => bucket.push(*word),
            StepOperand::Register(_) | StepOperand::Words(_) | StepOperand::File => {
                return Err(fault(encoding));
            }
        }
    }
    if bucket.len() != columns.0.len() {
        return Err(fault(encoding));
    }
    let site = index(encoding, source, &columns.0, None)?;
    let mut constants = Vec::with_capacity(bucket.len());
    for word in bucket {
        constants.push(WordSlot(constant(encoding, word)?.0));
    }
    if let [constant] = constants.as_slice() {
        size_site(encoding, source, &columns.0, *constant)?;
    }
    let constants = push_list(&mut encoding.code.pool, &constants);
    Ok(stage_scan(ScanWord {
        flag: WordFlag(flag | BUCKET.0),
        out,
        constants,
        read: WordSlot(site.0),
    }))
}

/// The word of a `Probe`: the batch column that holds its one value, or its operands in the pool with its range's ends after
/// them.
fn probe_word(encoding: &mut Encoding<'_>, read: ProbeRead<'_>) -> Result<CodeWord, LoweringError> {
    if read.columns.len() != read.values.len() {
        return Err(fault(encoding));
    }
    let site = index(
        encoding,
        read.source,
        read.columns,
        read.range.map(|range| range.column),
    )?;
    let mode = mode_code(read.mode).0;
    let one = match (read.values, read.range) {
        ([value], None) => register_of(value),
        ([] | [_, ..], Some(_)) | ([] | [_, _, ..], None) => None,
    };
    if let Some(register) = one {
        return Ok(stage_probe(ProbeWord {
            flag: WordFlag(mode),
            values: short_of(encoding, Count(1), PastWidth::WordOperands)?,
            out: read.out,
            probed: WordSlot(register.0),
            index: site,
        }));
    }
    let values = pooled_each(encoding, read.values)?;
    let (mut flag, mut ends) = (mode | PROBE_MANY.0, Vec::new());
    if let Some(range) = read.range {
        flag |= PROBE_RANGE.0;
        for (bound, inclusive) in [
            (&range.low, PROBE_LOW_INCLUSIVE),
            (&range.high, PROBE_HIGH_INCLUSIVE),
        ] {
            flag |= match bound.end {
                BoundKind::Inclusive => inclusive.0,
                BoundKind::Strict => 0,
            };
            ends.push(pooled(encoding, &bound.value)?);
        }
    }
    let at = push_operands(encoding, &values);
    for end in ends {
        push_operand(&mut encoding.code.pool, end);
    }
    Ok(stage_probe(ProbeWord {
        flag: WordFlag(flag),
        values: short_of(encoding, Count(values.len()), PastWidth::WordOperands)?,
        out: read.out,
        probed: WordSlot(at.0),
        index: site,
    }))
}

/// The word of a stage `Read`: one column of a row by its handle, from the stage's source.
fn read_word(
    encoding: &mut Encoding<'_>,
    source: StepRows,
    column: ReadColumn,
    row: Register,
    out: Register,
) -> Result<CodeWord, LoweringError> {
    let agrees = match (column, source) {
        (ReadColumn::Node(_), StepRows::Node(_))
        | (ReadColumn::Filled(_), StepRows::Filled(_))
        | (
            ReadColumn::Made(_),
            StepRows::Made(_) | StepRows::Written(_) | StepRows::Definition(_),
        ) => true,
        (
            ReadColumn::Node(_),
            StepRows::Filled(_)
            | StepRows::Made(_)
            | StepRows::Written(_)
            | StepRows::Definition(_),
        )
        | (
            ReadColumn::Filled(_),
            StepRows::Node(_) | StepRows::Made(_) | StepRows::Written(_) | StepRows::Definition(_),
        )
        | (ReadColumn::Made(_), StepRows::Node(_) | StepRows::Filled(_)) => false,
    };
    if !agrees {
        return Err(fault(encoding));
    }
    Ok(stage_read(StageReadWord {
        flag: source_code(source),
        column: column_code(encoding, column)?,
        row,
        out,
        read: source_number(encoding, source)?.number,
    }))
}

/// The words of a stage: one arm for each stage, each one word but a `Use` of a pattern in table form.
fn stage_words(
    encoding: &mut Encoding<'_>,
    stage: &FillStage,
    place: WordSlot,
    words: &mut Vec<PlacedWord>,
) -> Result<(), LoweringError> {
    let word = match stage {
        FillStage::Scan {
            source,
            columns,
            values,
            out,
        } => scan_word(encoding, *source, columns, values, *out)?,
        FillStage::Probe {
            source,
            columns,
            values,
            range,
            mode,
            out,
        } => probe_word(
            encoding,
            ProbeRead {
                source: *source,
                columns: &columns.0,
                values: &values.0,
                range: range.as_ref(),
                mode: *mode,
                out: *out,
            },
        )?,
        FillStage::Product { source, out } => stage_product(ProductWord {
            flag: source_code(*source),
            out: *out,
            read: source_number(encoding, *source)?.number,
        }),
        FillStage::Use {
            definition,
            bound,
            collected,
            values,
            mode,
            outputs,
        } => {
            let used = StageUse {
                definition: *definition,
                bound,
                collected,
                values,
                mode: *mode,
                outputs,
            };
            return use_words(encoding, used, place, words);
        }
        FillStage::Latest {
            table,
            key,
            bound,
            side,
            out,
        } => stage_latest(latest_operands(
            encoding,
            NearestRows {
                table: *table,
                key,
                bound,
                side: *side,
                out: *out,
            },
        )?),
        FillStage::Read {
            source,
            column,
            row,
            out,
        } => read_word(encoding, *source, *column, *row, *out)?,
        FillStage::Keep { test, left, right } => {
            stage_keep(test_operands(encoding, *test, left, right)?)
        }
        FillStage::Call {
            meaning,
            arguments,
            outputs,
        } => stage_call(call_operands(encoding, *meaning, arguments, outputs)?),
        FillStage::Render { node, context, out } => {
            stage_render(render_operands(*node, *context, *out))
        }
        FillStage::Walk {
            relation,
            walk,
            start,
            out,
        } => {
            let (flag, field) = match walk {
                TreeWalk::Children(field) => (0, field.unwrap_or(NO_FIELD)),
                TreeWalk::Axis(direction) => {
                    (WALK_AXIS.0 | (direction_code(*direction).0 << 1), NO_FIELD)
                }
            };
            stage_walk(WalkWord {
                flag: WordFlag(flag | extent_flag(StepRows::Node(*relation)).0),
                field,
                start: *start,
                out: *out,
            })
        }
    };
    words.push(PlacedWord { word, point: place });
    Ok(())
}

/// Writes a pipeline of a table's fill, answering where it starts: its stages, then the write of its row, which drops a row
/// the table holds when `repeats` says so.
pub(crate) fn pipeline_entry(
    encoding: &mut Encoding<'_>,
    table: MadeTableId,
    pipeline: &Pipeline,
    repeats: RepeatedRows,
) -> Result<CodeEntry, LoweringError> {
    let mut words = Vec::with_capacity(pipeline.stages.len().saturating_add(1));
    let mut place = PipelinePlace::First;
    for each in &pipeline.stages {
        held(encoding, &each.stage, place)?;
        place = PipelinePlace::Later;
        let counted = point(encoding, each.point)?;
        stage_words(encoding, &each.stage, counted, &mut words)?;
    }
    let flag = match repeats {
        RepeatedRows::Dropped => REPEATS_DROPPED,
        RepeatedRows::Kept => WordFlag(0),
    };
    let write = WriteWord {
        flag,
        columns: short_of(
            encoding,
            Count(pipeline.row.0.len()),
            PastWidth::TableColumns,
        )?,
        table,
        operands: operands_place(encoding, &pipeline.row)?,
    };
    words.push(PlacedWord {
        word: stage_write(write),
        point: NO_SLOT,
    });
    placed(encoding, &words)
}
