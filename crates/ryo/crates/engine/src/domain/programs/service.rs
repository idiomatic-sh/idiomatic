//! The `programs` service: what the host reads of a rule set's programs.

use super::types::{Programs, ProgramsEngine};
use crate::domain::code::constants::{STORAGE, STRAIGHT_WORDS};
use crate::domain::code::controlling::service::{exists_of, or_of};
use crate::domain::code::deciding::service::call_of;
use crate::domain::code::filling::service::stage_use_of;
use crate::domain::code::records::service::{body_at, operand_at};
use crate::domain::code::records::types::{BodyRecord, PooledOperand, ReadBody};
use crate::domain::code::types::{CodeEntry, CodeWord, Op, PoolAt, ProgramCode, UnitKey};
use crate::domain::code::walking::service::{closure_of, nearest_of};
use crate::domain::findings::types::RuleColumns;
use crate::domain::made_tables::types::{FillStore, KeyFrom, RuleSetView};
use crate::domain::memos::types::CellSpan;
use crate::domain::scratch::kept::types::QuickBody;
use crate::domain::scratch::types::{
    AskPart, DecodedCode, LoopState, Measuring, ScanScratch, ScratchEngine, UnitInFile,
};
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::service::file_words;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::FieldCode;
use ryo_language::inbound::use_cases::walk::types::RelationNeeds;
use ryo_relations::domain::relations::types::{Count, KindBits};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The relations some program of the rule set reads: what the host hands every walk, so a language builds only what is read.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::needs;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(needs(&Programs::default()).0.is_empty());
/// ```
#[must_use]
pub fn needs(programs: &Programs) -> RelationNeeds<'_> {
    RelationNeeds(&programs.0.reads)
}

/// The rules the rule set runs, in rule order: the ones `load` accepted.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::accepted;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(accepted(&Programs::default()).is_empty());
/// ```
#[must_use]
pub fn accepted(programs: &Programs) -> Vec<SelectedRuleOrdinal> {
    programs.0.code.rules.iter().map(|rule| rule.rule).collect()
}

/// What each column of each accepted rule's rows holds, in the order of [`accepted`]: what the host reads a found row by.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::columns;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(columns(&Programs::default()).is_empty());
/// ```
#[must_use]
pub fn columns(programs: &Programs) -> Vec<RuleColumns> {
    programs
        .0
        .code
        .rules
        .iter()
        .map(|rule| rule.columns.clone())
        .collect()
}

/// What the fill side reads of a rule set, built once per scan: its code, the words of its kinds and fields, and its
/// regular expressions.
pub(crate) fn rule_set(programs: &ProgramsEngine) -> RuleSetView<'_> {
    RuleSetView {
        code: &programs.code,
        kinds: &programs.kinds,
        fields: &programs.fields,
        regexes: &programs.regexes,
    }
}

/// How an `Exists` word asks its body: in place when it keeps no answer of its own and its body is inline, or a
/// pattern in call form whose program is a short straight line.
fn quick_of(code: &ProgramCode, word: CodeWord, body: Option<ReadBody>) -> QuickBody {
    if exists_of(word).flag.0 & STORAGE.0 != 0 {
        return QuickBody::Kept;
    }
    let word_at = |entry: CodeEntry, ahead: u32| {
        let place = usize::try_from(entry.0.wrapping_add(ahead)).unwrap_or(usize::MAX);
        code.words.get(place).map(|held| held.op)
    };
    match body.map(|read| read.record) {
        Some(BodyRecord::Inline { entry }) if word_at(entry, 0) == Some(Op::Yield) => {
            QuickBody::Row
        }
        Some(BodyRecord::Inline { entry }) => QuickBody::Inline(entry),
        Some(BodyRecord::Call { entry, .. }) => {
            // A straight line: each word gives one row at most, asks nothing and makes nothing, and the program ends
            // within the bound.
            let ends = (0..=STRAIGHT_WORDS.0).find(|ahead| word_at(entry, *ahead) == Some(Op::End));
            let straight = ends.is_some_and(|end| {
                (0..end).all(|ahead| {
                    matches!(
                        word_at(entry, ahead),
                        Some(
                            Op::Test
                                | Op::TestEqualConstant
                                | Op::TestNotEqualConstant
                                | Op::TestEqualRegister
                                | Op::TestNotEqualRegister
                                | Op::TestInConstants
                                | Op::Let
                                | Op::ReadNode
                                | Op::ReadFilled
                                | Op::ReadMade
                                | Op::Yield
                        )
                    )
                })
            });
            if straight {
                QuickBody::Straight(entry)
            } else {
                QuickBody::Kept
            }
        }
        Some(BodyRecord::Table { .. }) | None => QuickBody::Kept,
    }
}

/// Adds the parts of a call-form body's ask to the decoded code, once for each body record.
fn parts_added(code: &ProgramCode, body: Option<ReadBody>, decoded: &mut DecodedCode) {
    let Some(BodyRecord::Call { bound, formals, .. }) = body.map(|read| read.record) else {
        return;
    };
    let place = usize::try_from(formals.0).unwrap_or(usize::MAX);
    if decoded.parts_at.get(place).is_none_or(|start| start.0 != 0) {
        return;
    }
    let start = decoded.parts.len();
    let mut from = KeyFrom::Constants;
    for number in 0..bound.0 {
        let at = formals.0.wrapping_add(number.wrapping_mul(3));
        let formal = code.pool.get(usize::try_from(at).unwrap_or(usize::MAX));
        let value = operand_at(&code.pool, PoolAt(at.wrapping_add(1)));
        if matches!(value, Some(PooledOperand::Register(_))) {
            from = KeyFrom::Row;
        }
        decoded.parts.push(AskPart {
            formal: Register(formal.map_or(0, |slot| slot.0)),
            value,
            from: KeyFrom::Constants,
        });
    }
    for part in decoded.parts.iter_mut().skip(start) {
        part.from = from;
    }
    if let Some(slot) = decoded.parts_at.get_mut(place) {
        *slot = Count(start.saturating_add(1));
    }
}

/// What the loop reads of the code at a step in place of the pool: the body record of each word over one body, an
/// `Or`'s body records, a `Call` word's argument operands, and the parts of every call-form body's ask (the bodies the
/// loop's words and the fill side's `Use` stages name).
pub(crate) fn decoded_code(code: &ProgramCode) -> DecodedCode {
    let mut decoded = DecodedCode {
        bodies: Vec::with_capacity(code.words.len()),
        lists: Vec::with_capacity(code.words.len()),
        parts_at: vec![Count(0); code.pool.len()],
        ..DecodedCode::default()
    };
    let mut reads = Count(0);
    for word in &code.words {
        let (mut body, mut list) = (None, CellSpan::default());
        let mut quick = QuickBody::Kept;
        match word.op {
            Op::Exists => {
                body = body_at(&code.pool, exists_of(*word).body);
                quick = quick_of(code, *word, body);
            }
            Op::Nearest => body = body_at(&code.pool, nearest_of(*word).operands),
            Op::Closure => body = body_at(&code.pool, closure_of(*word).operands),
            Op::StageUse => parts_added(
                code,
                body_at(&code.pool, stage_use_of(*word).body),
                &mut decoded,
            ),
            Op::Or => {
                let asking = or_of(*word);
                let start = decoded.or_bodies.len();
                let mut at = Some(asking.records);
                for _ in 0..asking.bodies.0 {
                    let read = at.and_then(|place| body_at(&code.pool, place));
                    at = read.map(|held| held.after);
                    if let Some(held) = read {
                        decoded.or_bodies.push(held);
                        parts_added(code, Some(held), &mut decoded);
                    }
                }
                list = CellSpan {
                    start: Count(start),
                    length: Count(decoded.or_bodies.len().saturating_sub(start)),
                };
            }
            Op::Call => {
                let asking = call_of(*word);
                let start = decoded.operands.len();
                let count = code
                    .pool
                    .get(usize::try_from(asking.arguments.0).unwrap_or(usize::MAX))
                    .map_or(0, |held| held.0);
                for number in 0..count {
                    let at = asking
                        .arguments
                        .0
                        .wrapping_add(1)
                        .wrapping_add(number.wrapping_mul(2));
                    decoded.operands.push(operand_at(&code.pool, PoolAt(at)));
                }
                list = CellSpan {
                    start: Count(start),
                    length: Count(decoded.operands.len().saturating_sub(start)),
                };
            }
            Op::ReadFilled | Op::ReadMade => {
                // The word's slot among the columns the loop holds for a file.
                list = CellSpan {
                    start: reads,
                    length: Count(1),
                };
                reads = Count(reads.0.saturating_add(1));
            }
            Op::End
            | Op::EachKind
            | Op::EachMatchNode
            | Op::EachMatchFilled
            | Op::EachMatchFilledMany
            | Op::EachMatchMade
            | Op::EachRange
            | Op::EachRowNode
            | Op::EachRowFilled
            | Op::EachRowMade
            | Op::Children
            | Op::ChildrenOfField
            | Op::Axis
            | Op::Within
            | Op::NearestDerived
            | Op::ReadNode
            | Op::Test
            | Op::TestEqualConstant
            | Op::TestNotEqualConstant
            | Op::TestEqualRegister
            | Op::TestNotEqualRegister
            | Op::TestInConstants
            | Op::Render
            | Op::Let
            | Op::Fork
            | Op::Yield
            | Op::Latest
            | Op::Emit
            | Op::StageScan
            | Op::StageProbe
            | Op::StageLatest
            | Op::StageRead
            | Op::StageKeep
            | Op::StageCall
            | Op::StageProduct
            | Op::StageRender
            | Op::StageWalk
            | Op::StageWrite => {}
        }
        parts_added(code, body, &mut decoded);
        decoded.bodies.push(body);
        decoded.lists.push(list);
        decoded.quick.push(quick);
    }
    decoded
}

/// A worker's per-file state for `programs`: its registers, one for each the programs name, its words over the programs'
/// constants, and an empty store of fills.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::programs::service::scratch_for;
///
/// let _scratch = scratch_for(&Programs::default());
/// ```
#[must_use]
pub fn scratch_for(programs: &Programs) -> ScanScratch {
    let registers = usize::try_from(programs.0.code.counts.registers.0).unwrap_or(usize::MAX);
    ScanScratch(ScratchEngine {
        state: LoopState {
            registers: vec![CellWord(0); registers],
            decoded: decoded_code(&programs.0.code),
            ..LoopState::default()
        },
        store: FillStore::default(),
        words: file_words(&programs.0.words),
    })
}

/// A worker's per-file state for `programs` that also counts: the runs of each plan point, what each unit was asked and gave,
/// its time, and the sizes of every relation read, which it fills whole in every file. What it counted is read by
/// `counting::service::counted`. The counting is the executor's track's: until it counts, this is the plain state.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::scratch_measuring;
/// use ryo_engine::domain::programs::types::Programs;
///
/// let _scratch = scratch_measuring(&Programs::default());
/// ```
#[must_use]
pub fn scratch_measuring(programs: &Programs) -> ScanScratch {
    let mut scratch = scratch_for(programs);
    let code = &programs.0.code;
    scratch.0.state.measure = Some(Box::new(Measuring {
        points: vec![Count(0); usize::try_from(code.counts.points.0).unwrap_or(usize::MAX)],
        calls: vec![Count(0); code.units.len()],
        keys: vec![Count(0); code.units.len()],
        rows: vec![Count(0); code.units.len()],
        build: vec![Count(0); code.units.len()],
        nanos: vec![Count(0); code.units.len()],
        idle_nanos: vec![Count(0); code.units.len()],
        file: vec![UnitInFile::default(); code.units.len()],
        table_units: code.tables.iter().map(|table| table.unit).collect(),
        kinds: vec![Count(0); programs.0.kinds.len()],
        literals: vec![Count(0); code.sizes.len()],
        ..Measuring::default()
    }));
    scratch
}

/// The step of `rule`'s own plan whose rows the unit named `unit` makes: what a work notice names (`WorkNotice.step`). A unit
/// is named by a merged node, shared by every rule that reaches it; the step is the first of the rule's plan with that node.
/// A unit whose node is no step of the rule's plan (a cut that is one side of a step) answers the step the rule emits; a rule
/// these programs do not run, which has no notice, answers the first step.
#[must_use]
pub(crate) fn unit_step(programs: &Programs, rule: SelectedRuleOrdinal, unit: &UnitKey) -> StepId {
    let (UnitKey::Pattern(node, ..) | UnitKey::Rule(node) | UnitKey::Table(node, _)) = unit;
    let Some(steps) = programs.0.steps.iter().find(|steps| steps.rule == rule) else {
        return StepId(0);
    };
    let place = steps.nodes.0.iter().position(|each| each == node);
    match place.map(u32::try_from) {
        Some(Ok(place)) => StepId(place),
        // No step of the plan bears the node, or the plan has more steps than a step is numbered by.
        Some(Err(_)) | None => steps.emitted,
    }
}

/// The cell a node row's `Kind` column holds for a kind code: the word of the kind's name. None for a code the language's
/// vocabulary states no kind at.
#[must_use]
pub(crate) fn kind_word(programs: &Programs, kind: KindBits) -> Option<CellWord> {
    programs.0.kinds.get(usize::from(kind.0)).copied()
}

/// The cell a node row's `Field` column holds for a field code: the word of the field's name. Code 0, no field, is the
/// empty text's word. None for a code the language's vocabulary states no field at.
#[must_use]
pub(crate) fn field_word(programs: &Programs, field: FieldCode) -> Option<CellWord> {
    programs.0.fields.get(usize::from(field.0)).copied()
}
