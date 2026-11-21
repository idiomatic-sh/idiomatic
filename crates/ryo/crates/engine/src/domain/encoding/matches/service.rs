//! The services of the `matches` part: the words that give the rows of a source, each by the source it reads, and the read of
//! one cell of a row. A source and a column that do not go together are the encoding's fault, never a guess.

use crate::domain::code::anchoring::service::{
    each_kind, each_match_filled, each_match_filled_many, each_match_made, each_match_node,
    each_range, each_row_filled, each_row_made, each_row_node,
};
use crate::domain::code::anchoring::types::{
    EachKindWord, EachMatchMadeWord, EachMatchManyWord, EachMatchWord, EachRangeWord,
    EachRowFilledWord, EachRowMadeWord, EachRowNodeWord,
};
use crate::domain::code::constants::{
    CONSTANT, HIGH_INCLUSIVE, HIGH_PRESENT, LOW_INCLUSIVE, LOW_PRESENT, MANY, NO_SLOT, OWN_TREE,
    RANGE_OUT, SEVERAL, WRITTEN,
};
use crate::domain::code::deciding::service::{read_filled, read_made, read_node};
use crate::domain::code::deciding::types::{ReadFilledWord, ReadMadeWord, ReadNodeWord};
use crate::domain::code::records::service::{push_list, push_operand};
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::small_codes::service::node_column_code;
use crate::domain::code::types::{CodeWord, PoolAt, Short, SizeSite, WordFlag, WordSlot};
use crate::domain::encoding::programs::types::RangeRead;
use crate::domain::encoding::sites::service::{
    column_code, definition_table, fault, index, operands_place, pooled, pooled_each,
    push_operands, reach_table, relation, short_of, slot_value,
};
use crate::domain::encoding::types::{Encoding, FlaggedOperand, FlaggedSlot, SlotValue};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::types::{
    AnchorKinds, BoundKind, MatchColumns, RangeBound, ReadColumn, Register, StepOperands, StepRows,
};
use ryo_relations::domain::catalog::service::node_extent;
use ryo_relations::domain::relations::types::{Count, KindBits, NodeExtent, Relation};

/// The flag of the node rows a source covers: `OWN_TREE` when it reads the node table through a relation whose extent is the
/// file's own tree alone, so the rows of embedded segments are left out. Every word of a node source takes it from here.
pub(crate) fn extent_flag(source: StepRows) -> WordFlag {
    match source {
        StepRows::Node(relation) => match node_extent(relation) {
            Some(NodeExtent::Own) => OWN_TREE,
            // A relation over the embedded segments too; a filled relation read as node rows has no extent to hold to.
            Some(NodeExtent::Embedded) | None => WordFlag(0),
        },
        StepRows::Filled(_)
        | StepRows::Made(_)
        | StepRows::Written(_)
        | StepRows::Definition(_) => WordFlag(0),
    }
}

/// The word of an `EachKind` over the node rows of `relation`: one kind in the word, several in the pool.
pub(crate) fn kind_word(
    encoding: &mut Encoding<'_>,
    relation: Relation,
    kinds: &AnchorKinds,
    out: Register,
) -> CodeWord {
    let extent = extent_flag(StepRows::Node(relation));
    match &*kinds.0 {
        [one] => each_kind(EachKindWord {
            flag: extent,
            kind: *one,
            out,
            kinds: PoolAt(0),
        }),
        several => {
            let slots: Vec<WordSlot> = several
                .iter()
                .map(|kind| WordSlot(u32::from(kind.0)))
                .collect();
            let list = push_list(&mut encoding.code.pool, &slots);
            each_kind(EachKindWord {
                flag: WordFlag(extent.0 | SEVERAL.0),
                kind: KindBits(0),
                out,
                kinds: list,
            })
        }
    }
}

/// The one value a word holds in a slot, with the flag that says it is a constant; none when the values are not one that a
/// slot holds.
fn one_value(
    encoding: &mut Encoding<'_>,
    values: &StepOperands,
) -> Result<Option<FlaggedSlot>, LoweringError> {
    let [value] = &*values.0 else {
        return Ok(None);
    };
    Ok(match slot_value(encoding, value)? {
        Some(SlotValue::Register(slot)) => Some(FlaggedSlot {
            flag: WordFlag(0),
            slot,
        }),
        Some(SlotValue::Constant(slot)) => Some(FlaggedSlot {
            flag: CONSTANT,
            slot,
        }),
        None => None,
    })
}

/// Notes a bucket of a filled relation that one constant names in one column: a size a measuring scratch counts, once.
pub(crate) fn size_site(
    encoding: &mut Encoding<'_>,
    source: StepRows,
    columns: &[ReadColumn],
    constant: WordSlot,
) -> Result<(), LoweringError> {
    let (StepRows::Filled(read), [column]) = (source, columns) else {
        return Ok(());
    };
    let site = SizeSite {
        relation: WordSlot(relation(encoding, read)?.0),
        column: column_code(encoding, *column)?,
        value: constant,
    };
    if encoding.sizes.insert(site) {
        encoding.code.sizes.push(site);
    }
    Ok(())
}

/// The word of an `EachMatch`: each row of a source whose columns hold the values, through the index of the source by them.
pub(crate) fn match_word(
    encoding: &mut Encoding<'_>,
    source: StepRows,
    columns: &MatchColumns,
    values: &StepOperands,
    out: Register,
) -> Result<CodeWord, LoweringError> {
    if columns.0.len() != values.0.len() {
        return Err(fault(encoding));
    }
    let written = match source {
        StepRows::Written(_) => WRITTEN.0,
        StepRows::Node(_) | StepRows::Filled(_) | StepRows::Made(_) => 0,
        // A definition's rows are read whole, never matched: a keyed read of one is a use.
        StepRows::Definition(_) => return Err(fault(encoding)),
    };
    let index = index(encoding, source, &columns.0, None)?;
    let one = one_value(encoding, values)?;
    if let Some(FlaggedSlot { flag, slot }) = one
        && flag == CONSTANT
    {
        size_site(encoding, source, &columns.0, slot)?;
    }
    Ok(match (source, one) {
        (StepRows::Node(_), Some(FlaggedSlot { flag, slot: value })) => {
            each_match_node(EachMatchWord {
                flag: WordFlag(flag.0 | extent_flag(source).0),
                out,
                value,
                index,
            })
        }
        // An `EachMatch` over the node table has exactly one column, and its value is a register or a constant.
        (StepRows::Node(_) | StepRows::Definition(_), None)
        | (StepRows::Definition(_), Some(_)) => {
            return Err(fault(encoding));
        }
        (StepRows::Filled(_), Some(FlaggedSlot { flag, slot: value })) => {
            each_match_filled(EachMatchWord {
                flag,
                out,
                value,
                index,
            })
        }
        (StepRows::Filled(_), None) => each_match_filled_many(EachMatchManyWord {
            values: short_of(encoding, Count(values.0.len()), PastWidth::WordOperands)?,
            out,
            operands: operands_place(encoding, values)?,
            index,
        }),
        (StepRows::Made(_) | StepRows::Written(_), Some(FlaggedSlot { flag, slot: value })) => {
            each_match_made(EachMatchMadeWord {
                flag: WordFlag(flag.0 | written),
                values: Short(0),
                out,
                value,
                index,
            })
        }
        (StepRows::Made(_) | StepRows::Written(_), None) => each_match_made(EachMatchMadeWord {
            flag: WordFlag(MANY.0 | written),
            values: short_of(encoding, Count(values.0.len()), PastWidth::WordOperands)?,
            out,
            value: WordSlot(operands_place(encoding, values)?.0),
            index,
        }),
    })
}

/// One end of a range as the flag and the pool hold it: whether it is there, whether a row at it is in the range, its operand.
fn range_end(
    encoding: &mut Encoding<'_>,
    end: Option<&RangeBound>,
    present: WordFlag,
    inclusive: WordFlag,
) -> Result<FlaggedOperand, LoweringError> {
    Ok(match end {
        Some(bound) => {
            let held = match bound.end {
                BoundKind::Inclusive => present.0 | inclusive.0,
                BoundKind::Strict => present.0,
            };
            FlaggedOperand {
                flag: WordFlag(held),
                operand: pooled(encoding, &bound.value)?,
            }
        }
        // An absent end holds no value: its place in the pool names no register.
        None => FlaggedOperand {
            flag: WordFlag(0),
            operand: PooledOperand::Register(Register(NO_SLOT.0)),
        },
    })
}

/// The word of an `EachRange`: each row of a bucket whose range column lies between the bounds, through the index of the
/// source by the match columns, sorted by the range column.
pub(crate) fn range_word(
    encoding: &mut Encoding<'_>,
    read: RangeRead<'_>,
) -> Result<CodeWord, LoweringError> {
    if read.columns.0.len() != read.values.0.len() {
        return Err(fault(encoding));
    }
    match read.source {
        StepRows::Node(_) | StepRows::Filled(_) | StepRows::Made(_) | StepRows::Written(_) => {}
        StepRows::Definition(_) => return Err(fault(encoding)),
    }
    let index = index(encoding, read.source, &read.columns.0, Some(read.range))?;
    let values = pooled_each(encoding, &read.values.0)?;
    let low = range_end(encoding, read.low, LOW_PRESENT, LOW_INCLUSIVE)?;
    let high = range_end(encoding, read.high, HIGH_PRESENT, HIGH_INCLUSIVE)?;
    let (out_flag, range_out) = match read.range_out {
        Some(register) => (RANGE_OUT.0, WordSlot(register.0)),
        None => (0, NO_SLOT),
    };
    let operands = push_operands(encoding, &values);
    push_operand(&mut encoding.code.pool, low.operand);
    push_operand(&mut encoding.code.pool, high.operand);
    encoding.code.pool.push(range_out);
    Ok(each_range(EachRangeWord {
        flag: WordFlag(low.flag.0 | high.flag.0 | out_flag | extent_flag(read.source).0),
        values: short_of(encoding, Count(values.len()), PastWidth::WordOperands)?,
        out: read.out,
        operands,
        index,
    }))
}

/// The word of an `EachRow`: every row of a source; a definition's rows are its table's.
pub(crate) fn row_word(
    encoding: &mut Encoding<'_>,
    source: StepRows,
    out: Register,
) -> Result<CodeWord, LoweringError> {
    Ok(match source {
        StepRows::Node(_) => each_row_node(EachRowNodeWord {
            flag: extent_flag(source),
            out,
        }),
        StepRows::Filled(read) => each_row_filled(EachRowFilledWord {
            out,
            relation: relation(encoding, read)?,
        }),
        StepRows::Made(table) => {
            reach_table(encoding, table)?;
            each_row_made(EachRowMadeWord {
                flag: WordFlag(0),
                out,
                table,
            })
        }
        StepRows::Written(table) => {
            reach_table(encoding, table)?;
            each_row_made(EachRowMadeWord {
                flag: WRITTEN,
                out,
                table,
            })
        }
        StepRows::Definition(definition) => {
            let table = definition_table(encoding, definition)?;
            each_row_made(EachRowMadeWord {
                flag: WordFlag(0),
                out,
                table,
            })
        }
    })
}

/// The word of a `Read`: one column of a row by its handle. The column's variant is its source's.
pub(crate) fn read_word(
    encoding: &mut Encoding<'_>,
    source: StepRows,
    column: ReadColumn,
    row: Register,
    out: Register,
) -> Result<CodeWord, LoweringError> {
    Ok(match (column, source) {
        (ReadColumn::Node(node), StepRows::Node(_)) => read_node(ReadNodeWord {
            column: node_column_code(node),
            row,
            out,
        }),
        (ReadColumn::Filled(_), StepRows::Filled(read)) => read_filled(ReadFilledWord {
            place: column_code(encoding, column)?,
            row,
            out,
            relation: relation(encoding, read)?,
        }),
        (ReadColumn::Made(made), StepRows::Made(table) | StepRows::Written(table)) => {
            reach_table(encoding, table)?;
            read_made(ReadMadeWord {
                column: made,
                row,
                out,
                table,
            })
        }
        (ReadColumn::Made(made), StepRows::Definition(definition)) => {
            let table = definition_table(encoding, definition)?;
            read_made(ReadMadeWord {
                column: made,
                row,
                out,
                table,
            })
        }
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
        | (ReadColumn::Made(_), StepRows::Node(_) | StepRows::Filled(_)) => {
            return Err(fault(encoding));
        }
    })
}
