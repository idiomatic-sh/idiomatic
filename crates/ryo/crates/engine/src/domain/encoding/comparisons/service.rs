//! The services of the `comparisons` part: the operands of the words a program and a pipeline write alike, so a step and the
//! stage of the same meaning cannot be laid out differently.

use crate::domain::code::anchoring::types::LatestWord;
use crate::domain::code::constants::{BOUND_CONSTANT, LEFT_CONSTANT, RIGHT_CONSTANT, RIGHT_SET};
use crate::domain::code::deciding::types::{CallWord, RenderWord, TestWord};
use crate::domain::code::records::service::next_place;
use crate::domain::code::small_codes::service::{
    comparison_code, comparison_regex, context_code, meaning_code, meaning_flag, meaning_regex,
    side_code,
};
use crate::domain::code::types::{WordFlag, WordSlot};
use crate::domain::encoding::programs::types::NearestRows;
use crate::domain::encoding::sites::service::{
    constants_list, fault, index, pooled_each, push_operands, registers_list, short_of, slot_value,
};
use crate::domain::encoding::types::{Encoding, SlotValue};
use crate::domain::lowered::types::{MadeFill, OrderedFill};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::calls::service::call_held;
use crate::domain::steps::calls::types::CallForm;
use crate::domain::steps::types::{
    CallMeaning, ReadColumn, RegexId, Register, RegisterList, StepOperand, StepOperands, StepRows,
    TestOperation,
};
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::Count;

/// The regular expression a word names when its operation has none.
const NO_REGEX: RegexId = RegexId(0);

/// The operands of a test of two operands: the left a register or a constant, the right a register, a constant or a set.
pub(crate) fn test_operands(
    encoding: &mut Encoding<'_>,
    test: TestOperation,
    left: &StepOperand,
    right: &StepOperand,
) -> Result<TestWord, LoweringError> {
    let (mut flag, left) = match slot_value(encoding, left)? {
        Some(SlotValue::Register(slot)) => (0, slot),
        Some(SlotValue::Constant(slot)) => (LEFT_CONSTANT.0, slot),
        None => return Err(fault(encoding)),
    };
    let right = match right {
        StepOperand::Words(words) => {
            flag |= RIGHT_SET.0;
            WordSlot(constants_list(encoding, words)?.0)
        }
        StepOperand::Register(_) | StepOperand::Word(_) | StepOperand::File => {
            match slot_value(encoding, right)? {
                Some(SlotValue::Register(slot)) => slot,
                Some(SlotValue::Constant(slot)) => {
                    flag |= RIGHT_CONSTANT.0;
                    slot
                }
                None => return Err(fault(encoding)),
            }
        }
    };
    Ok(TestWord {
        flag: WordFlag(flag),
        comparison: comparison_code(test),
        left,
        right,
        regex: comparison_regex(test).unwrap_or(NO_REGEX),
    })
}

/// The operands of a call of a per-row meaning: the list of its arguments, the list of its outputs. A call whose arguments
/// and outputs are not its meaning's shape (`calls::service::call_held`) is the encoding's fault of the rule being written.
pub(crate) fn call_operands(
    encoding: &mut Encoding<'_>,
    meaning: CallMeaning,
    arguments: &StepOperands,
    outputs: &RegisterList,
) -> Result<CallWord, LoweringError> {
    match call_held(meaning, Count(arguments.0.len()), Count(outputs.0.len())) {
        CallForm::Stated => {}
        CallForm::Unstated => return Err(fault(encoding)),
    }
    let ready = pooled_each(encoding, &arguments.0)?;
    let count = WordSlot(u32::from(
        short_of(encoding, Count(ready.len()), PastWidth::WordOperands)?.0,
    ));
    let at = next_place(&encoding.code.pool);
    encoding.code.pool.push(count);
    let _operands = push_operands(encoding, &ready);
    Ok(CallWord {
        flag: meaning_flag(meaning),
        meaning: meaning_code(meaning),
        arguments: at,
        outputs: registers_list(encoding, outputs),
        regex: meaning_regex(meaning).unwrap_or(NO_REGEX),
    })
}

/// The operands of a render of a node in a syntax role.
#[must_use]
pub(crate) const fn render_operands(
    node: Register,
    context: RenderContext,
    out: Register,
) -> RenderWord {
    RenderWord {
        context: context_code(context),
        node,
        out,
    }
}

/// The operands of a read of the rows of an ordered table nearest a bound in a key group: the table's index by its fill's key
/// columns, sorted by its fill's order column, then the key's cells.
pub(crate) fn latest_operands(
    encoding: &mut Encoding<'_>,
    read: NearestRows<'_>,
) -> Result<LatestWord, LoweringError> {
    let pack = encoding.pack;
    let table = pack
        .tables
        .get(usize::try_from(read.table.0).unwrap_or(usize::MAX));
    let Some(MadeFill::Ordered(OrderedFill::Sorted(input))) = table.map(|table| &table.fill) else {
        return Err(fault(encoding));
    };
    let columns: Vec<ReadColumn> = input
        .keys
        .0
        .iter()
        .map(|column| ReadColumn::Made(*column))
        .collect();
    let site = index(
        encoding,
        StepRows::Made(read.table),
        &columns,
        input.order.map(ReadColumn::Made),
    )?;
    let (constant, bound) = match slot_value(encoding, read.bound)? {
        Some(SlotValue::Register(slot)) => (0, slot),
        Some(SlotValue::Constant(slot)) => (BOUND_CONSTANT.0, slot),
        None => return Err(fault(encoding)),
    };
    let key = pooled_each(encoding, &read.key.0)?;
    let at = next_place(&encoding.code.pool);
    encoding.code.pool.push(WordSlot(site.0));
    let _operands = push_operands(encoding, &key);
    Ok(LatestWord {
        flag: WordFlag(side_code(read.side).0 | constant),
        keys: short_of(encoding, Count(key.len()), PastWidth::WordOperands)?,
        out: read.out,
        bound,
        operands: at,
    })
}
