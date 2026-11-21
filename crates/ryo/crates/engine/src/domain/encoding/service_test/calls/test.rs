//! The calls the encoding writes: every call the contract states, as a step of a program and as a stage of a pipeline, reads
//! back from its word as its meaning with the arguments and outputs written; a call that is not its meaning's shape is the
//! encoding's fault of the rule being written.

use super::super::fixtures::{anchor, of, operands, pack, regs, rule, scan, table_pack, word};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::deciding::service::call_of;
use crate::domain::code::deciding::types::CallWord;
use crate::domain::code::filling::service::stage_call_of;
use crate::domain::code::records::service::operand_at;
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::service::list;
use crate::domain::code::small_codes::service::meaning_of;
use crate::domain::code::types::{CodeWord, Op, PoolAt, ProgramCode, WordSlot};
use crate::domain::encoding::constants::LAST_STEP;
use crate::domain::encoding::service::encoded;
use crate::domain::lowered::types::LoweredPack;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::LoadRefusal;
use crate::domain::steps::calls::fixtures::{ContractCall, contract_calls};
use crate::domain::steps::calls::types::{SearchGroup, SearchLines, SourceSearch};
use crate::domain::steps::stages::types::FillStage;
use crate::domain::steps::types::{CallMeaning, ProgramStep, RegexId, StepOperand};
use ryo_language::domain::facts::types::CellWord;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// The pack a row encodes.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Every call of the contract's fixture, as the steps of one rule and as the stages of one pipeline.
    EveryCall,
    /// A `Lower` of two texts, in a program.
    ArgumentTooMany,
    /// A `SpanBounds` that gives one cell, in a pipeline.
    OutputTooFew,
    /// A search that asks its group's line and numbers no lines.
    UnnumberedGroupLine,
    /// A `FitsRendered` of five arguments.
    UnevenFit,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = LoweringError;

/// The encoding's own fault of the first rule.
const FAULT: LoweringError = LoweringError::Refused {
    rule: SelectedRuleOrdinal(0),
    refusal: LoadRefusal::Unlowerable(LAST_STEP),
};

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_call_the_contract_states_reads_back_from_its_word"),
        word: Word::Succeeds,
        input: || Scenario::EveryCall,
        expect: Expect::Value(Cow::Borrowed(
            "17 Call words, 17 of 17 read back; 17 StageCall words, 17 of 17 read back",
        )),
    },
    Case {
        name: CaseName("a_call_with_an_argument_its_meaning_does_not_take_is_refused"),
        word: Word::Refuses,
        input: || Scenario::ArgumentTooMany,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_stage_call_without_an_output_its_meaning_gives_is_refused"),
        word: Word::Refuses,
        input: || Scenario::OutputTooFew,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_search_that_asks_its_groups_line_and_numbers_no_lines_is_refused"),
        word: Word::Refuses,
        input: || Scenario::UnnumberedGroupLine,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_fit_whose_texts_and_literals_do_not_interleave_is_refused"),
        word: Word::Refuses,
        input: || Scenario::UnevenFit,
        expect: Expect::Error(FAULT),
    },
];

/// A rule that anchors and then makes each call, one step each.
fn called(calls: &[ContractCall]) -> LoweredPack {
    let mut steps = vec![anchor()];
    steps.extend(calls.iter().map(|call| ProgramStep::Call {
        meaning: call.meaning,
        arguments: call.arguments.clone(),
        outputs: call.outputs.clone(),
    }));
    pack(vec![rule(0, steps)])
}

/// A table whose one pipeline scans and then makes each call, one stage each.
fn stage_called(calls: &[ContractCall]) -> LoweredPack {
    let mut filled = vec![scan(1)];
    filled.extend(calls.iter().map(|call| FillStage::Call {
        meaning: call.meaning,
        arguments: call.arguments.clone(),
        outputs: call.outputs.clone(),
    }));
    table_pack(filled, 1, Vec::new())
}

/// An operand as the pool holds it, read back as the lowering wrote it: a constant by its value, a set by its values.
fn operand_read(code: &ProgramCode, at: PoolAt) -> Option<StepOperand> {
    let value = |number: u32| {
        code.constants
            .get(usize::try_from(number).unwrap_or(usize::MAX))
            .copied()
    };
    Some(match operand_at(&code.pool, at)? {
        PooledOperand::Register(register) => StepOperand::Register(register),
        PooledOperand::Constant(number) => StepOperand::Word(value(number.0)?),
        PooledOperand::Constants(at) => StepOperand::Words(
            list(&code.pool, WordSlot(at.0))
                .iter()
                .map(|number| value(number.0))
                .collect::<Option<Box<[CellWord]>>>()?,
        ),
        PooledOperand::File => StepOperand::File,
    })
}

/// Whether a call word reads back as `call`: its meaning by its code, expression and flag; its arguments, each operand two
/// slots after the list's length; its outputs.
fn reads_back(code: &ProgramCode, read: CallWord, call: &ContractCall) -> bool {
    let count = code
        .pool
        .get(usize::try_from(read.arguments.0).unwrap_or(usize::MAX))
        .map_or(u32::MAX, |slot| slot.0);
    let arguments: Option<Vec<StepOperand>> = (0..count)
        .map(|place| {
            let at = read
                .arguments
                .0
                .saturating_add(1)
                .saturating_add(place.saturating_mul(2));
            operand_read(code, PoolAt(at))
        })
        .collect();
    let outputs: Vec<u32> = list(&code.pool, WordSlot(read.outputs.0))
        .iter()
        .map(|slot| slot.0)
        .collect();
    let written: Vec<u32> = call.outputs.0.iter().map(|register| register.0).collect();
    meaning_of(read.meaning, read.regex, read.flag) == Some(call.meaning)
        && arguments.as_deref() == Some(&*call.arguments.0)
        && outputs == written
}

/// How many words of `op` a code holds, and how many of them read back as the fixture's calls, in order.
fn read_back(code: &ProgramCode, op: Op, read: fn(CodeWord) -> CallWord) -> String {
    let calls = contract_calls();
    let words: Vec<CallWord> = code
        .words
        .iter()
        .filter(|word| word.op == op)
        .map(|word| read(*word))
        .collect();
    let same = words
        .iter()
        .zip(&calls)
        .filter(|(word, call)| reads_back(code, **word, call))
        .count();
    format!(
        "{} {op:?} words, {same} of {} read back",
        words.len(),
        calls.len()
    )
}

/// A search that asks its group's line and numbers no lines, with the counts its shape states.
fn unnumbered_group_line() -> ProgramStep {
    ProgramStep::Call {
        meaning: CallMeaning::SourceMatches(
            RegexId(0),
            SourceSearch {
                lines: SearchLines::Unnumbered,
                group: SearchGroup::Named {
                    text: TextOutput::Omitted,
                    line: LineOutput::Appended,
                },
            },
        ),
        arguments: operands([of(1), word(7)]),
        outputs: regs([10, 11, 12]),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let one = |step| pack(vec![rule(0, vec![anchor(), step])]);
    let lowered = match scenario {
        Scenario::EveryCall => {
            let calls = contract_calls();
            let (steps, stages) = (encoded(&called(&calls))?, encoded(&stage_called(&calls))?);
            return Ok(Cow::Owned(format!(
                "{}; {}",
                read_back(&steps, Op::Call, call_of),
                read_back(&stages, Op::StageCall, stage_call_of)
            )));
        }
        Scenario::ArgumentTooMany => one(ProgramStep::Call {
            meaning: CallMeaning::Lower,
            arguments: operands([of(1), of(2)]),
            outputs: regs([10]),
        }),
        Scenario::OutputTooFew => table_pack(
            vec![
                scan(1),
                FillStage::Call {
                    meaning: CallMeaning::SpanBounds,
                    arguments: operands([of(1)]),
                    outputs: regs([10]),
                },
            ],
            1,
            Vec::new(),
        ),
        Scenario::UnnumberedGroupLine => one(unnumbered_group_line()),
        Scenario::UnevenFit => one(ProgramStep::Call {
            meaning: CallMeaning::FitsRendered,
            arguments: operands([of(1), of(2), word(7), word(8), word(9)]),
            outputs: regs([]),
        }),
    };
    encoded(&lowered).map(|_| Cow::Borrowed("written"))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
