//! The services of the `staging` part: one stage of a pipeline over one batch, of the kinds that give one row for one or
//! keep some rows. Each reads its word through the code's readers once for the batch, then calls the one loop of its kind
//! in `stages`; none makes a table, fills a relation, builds an index or runs a step program.

use super::types::{BatchFrame, KeepScratch};
use crate::domain::code::constants::{
    LEFT_CONSTANT, MODE, PROBE_HIGH_INCLUSIVE, PROBE_LOW_INCLUSIVE, PROBE_MANY, PROBE_RANGE,
    REPEATS_DROPPED, RIGHT_CONSTANT, RIGHT_SET,
};
use crate::domain::code::filling::service::{
    stage_keep_of, stage_probe_of, stage_read_of, stage_write_of,
};
use crate::domain::code::filling::types::ProbeWord;
use crate::domain::code::small_codes::service::{comparison_of, mode_of, node_column_of};
use crate::domain::code::types::{CodeWord, PoolAt, Short, WordFlag};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::{MissingCode, UnbuiltFill};
use crate::domain::made_tables::columns::types::HeldCells;
use crate::domain::made_tables::columns::types::{CellsLie, RowColumn};
use crate::domain::made_tables::operands::service::{
    column_of, constant_at, key_of, operand, set_of, value_of,
};
use crate::domain::made_tables::pipelines::types::{BatchLevel, OperandRun};
use crate::domain::made_tables::reading::service::{
    column_held, index_at, node_names, site_at, site_source, source_of, view_of,
};
use crate::domain::made_tables::service::{place_of, write, write_new};
use crate::domain::made_tables::types::{ReadSource, RuleSetView, TableRows};
use crate::domain::made_tables::writing::service::{settled, written, written_new};
use crate::domain::stages::nodes::service::node_column;
use crate::domain::stages::ranging::service::probe_keep_between;
use crate::domain::stages::service::{cell_test, keep, keep_in, probe_keep, read};
use crate::domain::stages::testing::service::keep_text;
use crate::domain::stages::types::{
    BatchKey, BatchValue, KeptWhen, KeyPart, KeyRange, NodeNames, TextMeans, TextTest,
};
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{BoundKind, Register, TestOperation};
use ryo_language::domain::facts::service::intern;
use ryo_language::domain::facts::types::{CellWord, FileWords};
use ryo_relations::domain::relations::types::NodeColumn;
use ryo_relations::domain::relations::types::{Count, FileId};
use ryo_relations::domain::terms::types::NameText;

/// The code names a word that is not what its place needs.
pub(crate) const WRONG_WORD: FilledError = FilledError::Code {
    named: MissingCode::Word,
};

/// The mode of a probe or of a use, from its word's flag.
///
/// # Errors
///
/// The flag holds no mode.
pub(crate) fn mode(flag: WordFlag) -> Result<UseMode, FilledError> {
    mode_of(WordFlag(flag.0 & MODE.0)).ok_or(WRONG_WORD)
}

/// What a node row's cells are read from besides the table.
pub(crate) fn names_of(frame: BatchFrame<'_>) -> NodeNames<'_> {
    node_names(frame.programs, frame.text, frame.source, frame.file)
}

/// `Read`: the cell of each row's column, by the handle the batch holds, into the word's out column.
///
/// # Errors
///
/// The word reads a table while it is written, or names a register or a column the code does not hold.
pub(crate) fn read_stage(
    frame: BatchFrame<'_>,
    words: &mut FileWords,
    texts: &mut Vec<CellWord>,
    word: CodeWord,
    level: &mut BatchLevel,
) -> Result<(), FilledError> {
    let stage = stage_read_of(word);
    let source = source_of(stage.flag, stage.read);
    let place = usize::try_from(stage.out.0).unwrap_or(usize::MAX);
    let mut out = level
        .columns
        .get_mut(place)
        .map(std::mem::take)
        .ok_or(WRONG_WORD)?;
    let handles = column_of(&level.columns, stage.row);
    let done = match source {
        ReadSource::Filled(_) | ReadSource::Made(_) => {
            read(
                view_of(frame.store, source, stage.column),
                handles,
                &mut out,
            );
            Ok(())
        }
        ReadSource::Nodes(_) => match node_column_of(stage.column) {
            Some(column) => {
                node_column(
                    frame.nodes,
                    names_of(frame),
                    words,
                    texts,
                    column,
                    handles,
                    &mut out,
                );
                Ok(())
            }
            None => Err(WRONG_WORD),
        },
        ReadSource::Written(_) => Err(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Written,
        }),
    };
    if let Some(column) = level.columns.get_mut(place) {
        *column = out;
    }
    done
}

/// Says whether the register a `Read` filled holds the file: it does when the column read is a made table's own file
/// column (the column no cell is kept for) or the node table's. A text test of such a register reads the file's path,
/// not the text of the word numbered like the file.
pub(crate) fn file_marked(frame: BatchFrame<'_>, word: CodeWord, files: &mut Vec<Count>) {
    let stage = stage_read_of(word);
    let source = source_of(stage.flag, stage.read);
    let out = Count(usize::try_from(stage.out.0).unwrap_or(usize::MAX));
    files.retain(|held| *held != out);
    let the_file = match source {
        ReadSource::Made(_) => matches!(
            column_held(frame.store, source, stage.column).cells,
            HeldCells::TableFile(_)
        ),
        ReadSource::Nodes(_) => node_column_of(stage.column) == Some(NodeColumn::File),
        ReadSource::Filled(_) | ReadSource::Written(_) => false,
    };
    if the_file {
        files.push(out);
    }
}

/// `Keep`: the rows of the batch the word's test holds for, as the batch's selection. A comparison of two cells and a
/// membership are decided on the cells; a test of a text reads the file's words.
///
/// # Errors
///
/// The word names an operand the code does not hold, or a test the fill side does not run yet.
pub(crate) fn keep_stage(
    frame: BatchFrame<'_>,
    words: &mut FileWords,
    word: CodeWord,
    level: &mut BatchLevel,
    scratch: KeepScratch<'_>,
) -> Result<(), FilledError> {
    let KeepScratch {
        set,
        compiled,
        matcher,
        verdicts,
        files,
    } = scratch;
    let stage = stage_keep_of(word);
    let code = frame.programs.code;
    let test = comparison_of(stage.comparison, stage.regex).ok_or(WRONG_WORD)?;
    let BatchLevel {
        columns,
        keep: kept,
        rows,
        ..
    } = level;
    let left_is_file = stage.flag.0 & LEFT_CONSTANT.0 == 0
        && files.contains(&Count(usize::try_from(stage.left.0).unwrap_or(usize::MAX)));
    let left = if stage.flag.0 & LEFT_CONSTANT.0 == 0 {
        BatchValue::Column(column_of(columns, Register(stage.left.0)))
    } else {
        BatchValue::Constant(constant_at(code, stage.left)?)
    };
    let right = if stage.flag.0 & RIGHT_SET.0 != 0 {
        set_of(code, stage.right, set)?;
        None
    } else if stage.flag.0 & RIGHT_CONSTANT.0 == 0 {
        Some(BatchValue::Column(column_of(
            columns,
            Register(stage.right.0),
        )))
    } else {
        Some(BatchValue::Constant(constant_at(code, stage.right)?))
    };
    // The file's text is its path: a test of a text whose left is the file reads the word the path is.
    let text_left = if left_is_file {
        BatchValue::Constant(intern(words, NameText(frame.path.0)))
    } else {
        left
    };
    match (cell_test(test), test, right) {
        (Some(cells), _, Some(right)) => {
            keep(cells, left, right, *rows, kept);
            Ok(())
        }
        (None, TestOperation::In, None) => {
            keep_in(left, set, KeptWhen::Present, *rows, kept);
            Ok(())
        }
        (None, TestOperation::NotIn, None) => {
            keep_in(left, set, KeptWhen::Absent, *rows, kept);
            Ok(())
        }
        (Some(_), _, None) | (None, TestOperation::In | TestOperation::NotIn, Some(_)) => {
            Err(WRONG_WORD)
        }
        (
            None,
            TestOperation::Matches(_)
            | TestOperation::Glob
            | TestOperation::NotGlob
            | TestOperation::StartsWith
            | TestOperation::EndsWith
            | TestOperation::NumericAtLeast
            | TestOperation::NaturalLess
            | TestOperation::Successor,
            right,
        ) => keep_text(
            TextMeans {
                matcher,
                regexes: frame.programs.regexes,
                compiled,
                verdicts,
                stamp: frame.store.stamp,
            },
            words,
            TextTest {
                test,
                left: text_left,
                right,
                set,
                set_at: (stage.flag.0 & RIGHT_SET.0 != 0).then_some(PoolAt(stage.right.0)),
            },
            *rows,
            kept,
        ),
        (
            None,
            TestOperation::Equal
            | TestOperation::NotEqual
            | TestOperation::Less
            | TestOperation::LessOrEqual
            | TestOperation::Greater
            | TestOperation::GreaterOrEqual,
            _,
        ) => Err(WRONG_WORD),
    }
}

/// The key a probe asks its index for each row: the one register, or its operands.
pub(crate) fn probe_key<'l>(
    frame: BatchFrame<'_>,
    stage: ProbeWord,
    columns: &'l [Vec<CellWord>],
    cells: &mut Vec<CellWord>,
    parts: &mut Vec<KeyPart<'l>>,
) -> Result<(), FilledError> {
    if stage.flag.0 & PROBE_MANY.0 == 0 {
        cells.clear();
        cells.push(CellWord(0));
        parts.clear();
        parts.push(KeyPart {
            place: Count(0),
            column: column_of(columns, Register(stage.probed.0)),
        });
        return Ok(());
    }
    key_of(
        columns,
        frame.programs.code,
        frame.file,
        OperandRun {
            first: PoolAt(stage.probed.0),
            count: Count(usize::from(stage.values.0)),
        },
        cells,
        parts,
    )
}

/// The range a probe is held to, when its word has one: the two bounds after its values, and the order column of its
/// index's source.
pub(crate) fn probe_range<'l>(
    frame: BatchFrame<'l>,
    stage: ProbeWord,
    columns: &'l [Vec<CellWord>],
) -> Result<Option<KeyRange<'l>>, FilledError> {
    if stage.flag.0 & PROBE_RANGE.0 == 0 {
        return Ok(None);
    }
    let code = frame.programs.code;
    let (first, values) = (PoolAt(stage.probed.0), usize::from(stage.values.0));
    let bound = |ahead: Count| {
        value_of(
            columns,
            code,
            frame.file,
            operand(code, first, Count(values.saturating_add(ahead.0)))?,
        )
    };
    let end = |inclusive: WordFlag| {
        if stage.flag.0 & inclusive.0 == 0 {
            BoundKind::Strict
        } else {
            BoundKind::Inclusive
        }
    };
    let site = site_at(code, stage.index)?;
    let order = Short(u16::try_from(site.ordered.0).map_err(|_| WRONG_WORD)?);
    Ok(Some(KeyRange {
        order: view_of(frame.store, site_source(code, site)?, order),
        low: bound(Count(0))?,
        low_end: end(PROBE_LOW_INCLUSIVE),
        high: bound(Count(1))?,
        high_end: end(PROBE_HIGH_INCLUSIVE),
    }))
}

/// `Probe` in present or absent mode: the rows of the batch whose key has a row in the index, or has none, as the
/// batch's selection.
///
/// # Errors
///
/// The word names an operand or a site the code does not hold.
pub(crate) fn probe_keep_stage(
    frame: BatchFrame<'_>,
    word: CodeWord,
    kept: KeptWhen,
    level: &mut BatchLevel,
    key: &mut Vec<CellWord>,
) -> Result<(), FilledError> {
    let stage = stage_probe_of(word);
    let BatchLevel {
        columns,
        keep: selection,
        rows,
        ..
    } = level;
    let mut parts = Vec::new();
    probe_key(frame, stage, columns, key, &mut parts)?;
    let asked = BatchKey {
        cells: key,
        parts: &parts,
    };
    let index = index_at(frame.store, stage.index);
    match probe_range(frame, stage, columns)? {
        None => probe_keep(index, asked, *rows, kept, selection),
        Some(range) => probe_keep_between(index, asked, range, *rows, kept, selection),
    }
    Ok(())
}

/// The write of a pipeline's row for every row of the batch: the word's operands, one for each column, appended to the
/// table; with repeats dropped, only the rows the table does not hold. A made table's rows go to the end of `arena`,
/// the store's one list of the file's tables; a fill's gathered input keeps its own list.
///
/// # Errors
///
/// The word names an operand the code does not hold.
pub(crate) fn write_stage(
    programs: RuleSetView<'_>,
    file: FileId,
    word: CodeWord,
    level: &BatchLevel,
    into: &mut TableRows,
    arena: &mut Vec<CellWord>,
) -> Result<(), FilledError> {
    let stage = stage_write_of(word);
    let code = programs.code;
    let width = usize::from(stage.columns.0);
    let mut sources = Vec::with_capacity(width);
    for place in 0..width {
        // The table keeps no cell of the file's own column.
        if place_of(into, Count(place)) == RowColumn::File {
            continue;
        }
        sources.push(value_of(
            &level.columns,
            code,
            file,
            operand(code, stage.operands, Count(place))?,
        )?);
    }
    let kept = stage.flag.0 & REPEATS_DROPPED.0 == 0;
    match (into.lies, kept) {
        (CellsLie::Own, true) => write(into, &sources, level.rows),
        (CellsLie::Own, false) => write_new(into, &sources, level.rows),
        (CellsLie::Arena(_), true) => {
            let _start = settled(arena, into);
            written(arena, into, &sources, level.rows);
        }
        (CellsLie::Arena(_), false) => {
            let start = settled(arena, into);
            written_new(arena, start, into, &sources, level.rows);
        }
    }
    Ok(())
}
