//! The services of the `programs` part: a step program written as a run of words that ends in `End`, each step by its one
//! word, with the programs it runs (inline bodies, the slots of a `Fork`) written before it. It holds where a file-sized word
//! may stand: only first in a rule's branch.

use crate::domain::code::anchoring::service::latest;
use crate::domain::code::constants::{
    ABSENT, CONSTANT, FRONTIER, KEPT, NO_SLOT, STORAGE_FILE_SLOT, STORAGE_KEYED_MAP, THE_FILE,
};
use crate::domain::code::controlling::service::{emit, end, exists, fork, or, yield_row};
use crate::domain::code::controlling::types::{EmitWord, ExistsWord, ForkWord, OrWord, YieldWord};
use crate::domain::code::deciding::service::{call, let_value, render, test};
use crate::domain::code::deciding::types::LetWord;
use crate::domain::code::records::service::{next_place, push_list};
use crate::domain::code::types::{CodeEntry, CodeWord, MemoSite, WordFlag, WordSlot};
use crate::domain::code::walking::service::{closure, nearest, nearest_derived};
use crate::domain::code::walking::types::{ClosureWord, NearestWord};
use crate::domain::encoding::bodies::service::{push_body, ready_body};
use crate::domain::encoding::comparisons::service::{
    call_operands, latest_operands, render_operands, test_operands,
};
use crate::domain::encoding::matches::service::{
    extent_flag, kind_word, match_word, range_word, read_word, row_word,
};
use crate::domain::encoding::programs::types::{
    DerivedEdges, KeptAnswer, NearestRows, NearestWalk, RangeRead,
};
use crate::domain::encoding::sites::service::{
    constant, fault, memo, operands_place, registers_list, short_of, slot_of,
};
use crate::domain::encoding::types::{Encoding, PlacedWord, ProgramRole};
use crate::domain::encoding::units::service::{placed, point};
use crate::domain::encoding::walks::service::{axis_word, children_word, within_word};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::service::step_class;
use crate::domain::steps::types::{
    CostClass, EmitTarget, NearestEdges, Polarity, ProgramStep, Register, RegisterList, StepBody,
    StepMemo, StepOperand, StepOperands, StepProgram, StepRows,
};
use ryo_relations::domain::relations::types::Count;

/// Writes a step program, answering where it starts. A file-sized step stands only first in a rule's branch: anywhere else,
/// and anywhere in a body, a pattern or a slot, it is the encoding's fault of the rule being written.
pub(crate) fn program_entry(
    encoding: &mut Encoding<'_>,
    program: &StepProgram,
    role: ProgramRole,
) -> Result<CodeEntry, LoweringError> {
    let mut words = Vec::with_capacity(program.0.len().saturating_add(1));
    for (place, each) in program.0.iter().enumerate() {
        let anchors = role == ProgramRole::Branch && place == 0;
        if step_class(&each.step) == CostClass::File && !anchors {
            return Err(fault(encoding));
        }
        let word = step_word(encoding, &each.step)?;
        words.push(PlacedWord {
            word,
            point: point(encoding, each.point)?,
        });
    }
    words.push(PlacedWord {
        word: end(),
        point: NO_SLOT,
    });
    placed(encoding, &words)
}

/// How the answer of a step over a body is kept: not at all, once per file when no register keys it, else by their values.
fn kept(encoding: &mut Encoding<'_>, kept: &StepMemo) -> Result<KeptAnswer, LoweringError> {
    Ok(match kept {
        StepMemo::None => KeptAnswer {
            storage: WordFlag(0),
            site: MemoSite(NO_SLOT.0),
            key: Vec::new(),
        },
        StepMemo::Keyed(registers) => KeptAnswer {
            storage: if registers.0.is_empty() {
                STORAGE_FILE_SLOT
            } else {
                STORAGE_KEYED_MAP
            },
            site: memo(encoding)?,
            key: registers
                .0
                .iter()
                .map(|register| WordSlot(register.0))
                .collect(),
        },
    })
}

/// The word of a `Nearest`: the bodies it runs written first, then its record: over derived edges the edges' body and the
/// register of the element whose edges are asked; then the condition's body, the register of the element under test, the memo.
fn nearest_word(
    encoding: &mut Encoding<'_>,
    walk: NearestWalk<'_>,
) -> Result<CodeWord, LoweringError> {
    let condition = ready_body(encoding, walk.condition, None, program_entry)?;
    let derived = match walk.edges {
        NearestEdges::Tree(_) => None,
        NearestEdges::Derived { edges, from } => Some(DerivedEdges {
            edges: ready_body(encoding, edges, None, program_entry)?,
            from: *from,
        }),
    };
    let answer = kept(encoding, walk.memo)?;
    let at = next_place(&encoding.code.pool);
    if let Some(derived) = &derived {
        let _record = push_body(encoding, &derived.edges);
        encoding.code.pool.push(WordSlot(derived.from.0));
    }
    let _record = push_body(encoding, &condition);
    encoding
        .code
        .pool
        .extend_from_slice(&[WordSlot(walk.element.0), WordSlot(answer.site.0)]);
    let kept = if answer.storage == WordFlag(0) {
        WordFlag(0)
    } else {
        KEPT
    };
    let flag = match walk.edges {
        NearestEdges::Tree(of) => WordFlag(kept.0 | extent_flag(StepRows::Node(*of)).0),
        NearestEdges::Derived { .. } => kept,
    };
    let operands = NearestWord {
        flag,
        start: walk.start,
        out: walk.out,
        operands: at,
    };
    Ok(match derived {
        Some(_) => nearest_derived(operands),
        None => nearest(operands),
    })
}

/// The word of an `Exists`: its body's record, then the registers that key its answer.
fn exists_word(
    encoding: &mut Encoding<'_>,
    polarity: Polarity,
    body: &StepBody,
    answer: &StepMemo,
) -> Result<CodeWord, LoweringError> {
    let ready = ready_body(encoding, body, None, program_entry)?;
    let answer = kept(encoding, answer)?;
    let record = push_body(encoding, &ready);
    let key = next_place(&encoding.code.pool);
    encoding.code.pool.extend_from_slice(&answer.key);
    let absent = match polarity {
        Polarity::Present => 0,
        Polarity::Absent => ABSENT.0,
    };
    Ok(exists(ExistsWord {
        flag: WordFlag(absent | answer.storage.0),
        keys: short_of(encoding, Count(answer.key.len()), PastWidth::WordOperands)?,
        body: record,
        key,
        memo: answer.site,
    }))
}

/// The word of an `Or`: its bodies' records in order, then the list of its outputs and the list of the registers that key it.
fn or_word(
    encoding: &mut Encoding<'_>,
    bodies: &[StepBody],
    outputs: &RegisterList,
    answer: &StepMemo,
) -> Result<CodeWord, LoweringError> {
    let ready: Result<Vec<_>, LoweringError> = bodies
        .iter()
        .map(|body| ready_body(encoding, body, Some(outputs), program_entry))
        .collect();
    let (ready, answer) = (ready?, kept(encoding, answer)?);
    let records = next_place(&encoding.code.pool);
    for body in &ready {
        let _record = push_body(encoding, body);
    }
    let outputs = registers_list(encoding, outputs);
    let _key = push_list(&mut encoding.code.pool, &answer.key);
    Ok(or(OrWord {
        flag: answer.storage,
        bodies: short_of(encoding, Count(ready.len()), PastWidth::WordOperands)?,
        records,
        outputs,
        memo: answer.site,
    }))
}

/// The word of a `Fork`: each slot's program written, then the list of their entries.
fn fork_word(
    encoding: &mut Encoding<'_>,
    slots: &[StepProgram],
) -> Result<CodeWord, LoweringError> {
    let entries: Result<Vec<WordSlot>, LoweringError> = slots
        .iter()
        .map(|slot| {
            program_entry(encoding, slot, ProgramRole::Keyed).map(|entry| WordSlot(entry.0))
        })
        .collect();
    let entries = entries?;
    Ok(fork(ForkWord {
        slots: short_of(encoding, Count(entries.len()), PastWidth::WordOperands)?,
        entries: push_list(&mut encoding.code.pool, &entries),
    }))
}

/// The word of a `Let`: a register takes a register's value, a constant or the file.
fn let_word(
    encoding: &mut Encoding<'_>,
    from: &StepOperand,
    out: Register,
) -> Result<CodeWord, LoweringError> {
    let (flag, from) = match from {
        StepOperand::Register(register) => (WordFlag(0), WordSlot(register.0)),
        StepOperand::Word(word) => (CONSTANT, WordSlot(constant(encoding, *word)?.0)),
        StepOperand::File => (THE_FILE, WordSlot(0)),
        StepOperand::Words(_) => return Err(fault(encoding)),
    };
    Ok(let_value(LetWord { flag, from, out }))
}

/// The word of an `Emit`: the rule, the frontier entry when the row is one of its, and the row's operands.
fn emit_word(
    encoding: &mut Encoding<'_>,
    target: EmitTarget,
    columns: &StepOperands,
) -> Result<CodeWord, LoweringError> {
    let (flag, rule, input) = match target {
        EmitTarget::Rule(rule) => (WordFlag(0), rule, WordSlot(0)),
        EmitTarget::Frontier(rule, input) => (
            FRONTIER,
            rule,
            slot_of(encoding, Count(input.0), PastWidth::WordOperands)?,
        ),
    };
    Ok(emit(EmitWord {
        flag,
        columns: short_of(encoding, Count(columns.0.len()), PastWidth::WordOperands)?,
        rule,
        input,
        operands: operands_place(encoding, columns)?,
    }))
}

/// The word of a step: one arm for each instruction.
fn step_word(encoding: &mut Encoding<'_>, step: &ProgramStep) -> Result<CodeWord, LoweringError> {
    Ok(match step {
        ProgramStep::EachKind {
            relation: of,
            kinds,
            out,
        } => kind_word(encoding, *of, kinds, *out),
        ProgramStep::EachMatch {
            source,
            columns,
            values,
            out,
        } => match_word(encoding, *source, columns, values, *out)?,
        ProgramStep::EachRange {
            source,
            columns,
            values,
            range,
            low,
            high,
            out,
            range_out,
        } => range_word(
            encoding,
            RangeRead {
                source: *source,
                columns,
                values,
                range: *range,
                low: low.as_ref(),
                high: high.as_ref(),
                out: *out,
                range_out: *range_out,
            },
        )?,
        ProgramStep::EachRow { source, out } => row_word(encoding, *source, *out)?,
        ProgramStep::Children {
            relation,
            parent,
            field,
            out,
        } => children_word(*relation, *parent, *field, *out),
        ProgramStep::Axis {
            relation,
            direction,
            start,
            out,
        } => axis_word(*relation, *direction, *start, *out),
        ProgramStep::Within {
            relation,
            ancestor,
            descendant,
        } => within_word(*relation, *ancestor, *descendant),
        ProgramStep::Nearest {
            edges,
            condition,
            element,
            start,
            out,
            memo,
        } => {
            let walk = NearestWalk {
                edges,
                condition,
                element: *element,
                start: *start,
                out: *out,
                memo,
            };
            nearest_word(encoding, walk)?
        }
        ProgramStep::Closure {
            edges,
            from,
            seed,
            out,
        } => {
            let ready = ready_body(encoding, edges, None, program_entry)?;
            let operands = push_body(encoding, &ready);
            encoding.code.pool.push(WordSlot(from.0));
            closure(ClosureWord {
                seed: *seed,
                out: *out,
                operands,
            })
        }
        ProgramStep::Read {
            source,
            column,
            row,
            out,
        } => read_word(encoding, *source, *column, *row, *out)?,
        ProgramStep::Test {
            test: tested,
            left,
            right,
        } => test(test_operands(encoding, *tested, left, right)?),
        ProgramStep::Call {
            meaning,
            arguments,
            outputs,
        } => call(call_operands(encoding, *meaning, arguments, outputs)?),
        ProgramStep::Render { node, context, out } => {
            render(render_operands(*node, *context, *out))
        }
        ProgramStep::Let { from, out } => let_word(encoding, from, *out)?,
        ProgramStep::Exists {
            polarity,
            body,
            memo,
        } => exists_word(encoding, *polarity, body, memo)?,
        ProgramStep::Or {
            bodies,
            outputs,
            memo,
        } => or_word(encoding, bodies, outputs, memo)?,
        ProgramStep::Fork { slots } => fork_word(encoding, slots)?,
        ProgramStep::Yield { row } => {
            let cells = short_of(encoding, Count(row.0.len()), PastWidth::WordOperands)?;
            let row = match &*row.0 {
                [one] => WordSlot(one.0),
                several => WordSlot(registers_list(encoding, &RegisterList(several.into())).0),
            };
            yield_row(YieldWord { cells, row })
        }
        ProgramStep::Latest {
            table,
            key,
            bound,
            side,
            out,
        } => latest(latest_operands(
            encoding,
            NearestRows {
                table: *table,
                key,
                bound,
                side: *side,
                out: *out,
            },
        )?),
        ProgramStep::Emit { target, columns } => emit_word(encoding, *target, columns)?,
    })
}
