//! The call's cases, through the loop, over fixture packs written as steps and encoded by the straight encoding. A rule
//! reads each cell of a table of constants and calls a per-row meaning with it. The rule set's base words are the fill
//! side's fixture texts (word 2 is `?d`, 3 is `ef`, 4 is `_`, 5 is `{a}b`, 6 is `{x}{y}`, 7 is `first`, 9 is `f`) and its
//! regular expressions the
//! fixture's (expression 1 is `[bd]`).

use super::service::call;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::Op;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::executing::service::run as run_branch;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::made_tables::fixtures::packs::{REGEXES, TEXTS, constants, of, word};
use crate::domain::programs::fixtures::{framed, held};
use crate::domain::programs::frames::types::RunMode;
use crate::domain::programs::operands::service::set;
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{
    CallMeaning, EmitTarget, MadeColumn, MadeTableId, PlacedStep, ProgramStep, ReadColumn, RegexId,
    Register, RegisterCount, RegisterList, StepOperand, StepOperands, StepProgram, StepRegex,
    StepRows,
};
use ryo_language::domain::facts::service::base_words;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::fixed_names::service::checked;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal as Rule;
use std::borrow::Cow;

/// The meaning a case's rule calls with each cell of the table.
#[derive(Clone, Copy, Debug)]
enum Called {
    /// The start and the end of a span: one row, two outputs.
    SpanBounds,
    /// The slots of the pattern `{x}{y}` matched against a text: a row for each way, two outputs.
    Capture,
    /// The slot of the pattern `{a}b`, which no text of the cases ends in: no row.
    CaptureNothing,
    /// The text with every occurrence of `[bd]` replaced by `_`: one row, one output.
    Replace,
    /// The text in lowercase: a meaning the kernel does not run yet.
    Lower,
}

/// The meaning, and the cells of the table, in order.
type Input = (Called, &'static [u64]);
/// The outputs of each row found; then 99, the rows the run answered, the words counted as not run, whether a fault was
/// kept (1), and how many cursors and staged cells are left; then, the word asked once more for the first cell, the
/// rows it answered and the cells it left staged.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// A span from one byte to another, as a cell.
const fn span(start: u64, end: u64) -> u64 {
    (start << 32) | end
}

/// Two spans, each from its start to a later end.
const IN_ORDER: &[u64] = &[span(3, 9), span(0, 2)];

/// A span that ends before it starts, then one in order.
const FIRST_OUT_OF_ORDER: &[u64] = &[span(9, 3), span(1, 2)];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_call_that_gives_one_row_binds_its_outputs_and_goes_on"),
        word: Word::Succeeds,
        input: || (Called::SpanBounds, IN_ORDER),
        expect: Expect::Value(Cow::Borrowed(&[3, 9, 0, 2, 99, 2, 0, 0, 0, 0, 1, 2])),
    },
    Case {
        name: CaseName("a_meanings_fault_on_a_row_ends_the_candidate_and_is_kept"),
        word: Word::Succeeds,
        input: || (Called::SpanBounds, FIRST_OUT_OF_ORDER),
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 99, 1, 0, 1, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_call_that_gives_several_rows_binds_each_in_turn"),
        word: Word::Succeeds,
        input: || (Called::Capture, &[7]),
        expect: Expect::Value(Cow::Borrowed(&[
            9, 10, 11, 12, 13, 14, 15, 16, 99, 4, 0, 0, 0, 0, 4, 8,
        ])),
    },
    Case {
        name: CaseName("a_call_that_gives_no_row_ends_the_candidate"),
        word: Word::Succeeds,
        input: || (Called::CaptureNothing, &[3, 2]),
        expect: Expect::Value(Cow::Borrowed(&[99, 0, 0, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_call_that_writes_a_text_gives_a_word_of_the_file"),
        word: Word::Succeeds,
        input: || (Called::Replace, &[2, 3]),
        expect: Expect::Value(Cow::Borrowed(&[10, 3, 99, 2, 0, 0, 0, 0, 1, 1])),
    },
    Case {
        name: CaseName("a_meaning_the_kernel_does_not_run_is_counted_and_ends_the_candidate"),
        word: Word::Succeeds,
        input: || (Called::Lower, &[3]),
        expect: Expect::Value(Cow::Borrowed(&[99, 0, 1, 0, 0, 0, 0, 0])),
    },
];

/// The rule set: the table of cells, one rule.
fn pack((called, cells): Input) -> LoweredPack {
    let (meaning, arguments, outputs): (CallMeaning, Vec<StepOperand>, &[u32]) = match called {
        Called::SpanBounds => (CallMeaning::SpanBounds, vec![of(1)], &[2, 3]),
        Called::Capture => (CallMeaning::Capture, vec![of(1), word(6)], &[2, 3]),
        Called::CaptureNothing => (CallMeaning::Capture, vec![of(1), word(5)], &[2]),
        Called::Replace => (CallMeaning::Replace(RegexId(1)), vec![of(1), word(4)], &[2]),
        Called::Lower => (CallMeaning::Lower, vec![of(1)], &[2]),
    };
    let registers =
        |each: &[u32]| RegisterList(each.iter().map(|number| Register(*number)).collect());
    let steps = vec![
        ProgramStep::EachRow {
            source: StepRows::Made(MadeTableId(0)),
            out: Register(0),
        },
        ProgramStep::Read {
            source: StepRows::Made(MadeTableId(0)),
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(0),
            out: Register(1),
        },
        ProgramStep::Call {
            meaning,
            arguments: StepOperands(arguments.into()),
            outputs: registers(outputs),
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: StepOperands(outputs.iter().map(|number| of(*number)).collect()),
        },
    ];
    LoweredPack {
        tables: vec![constants(cells)],
        rules: vec![LoweredRule {
            rule: SelectedRuleOrdinal(0),
            node: NodeDigest([1; 16]),
            branches: vec![Branch {
                emptiness: Emptiness::Unknown,
                program: StepProgram(
                    steps
                        .into_iter()
                        .map(|step| PlacedStep { point: None, step })
                        .collect(),
                ),
            }],
            columns: RuleColumns {
                emit: ColumnKinds(vec![CellKind::Number; outputs.len()]),
                frontier: Vec::new(),
            },
        }],
        registers: RegisterCount(4),
        ..LoweredPack::default()
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let code = encoded(&pack(input)).map_err(|_| ())?;
    let entry = code
        .rules
        .first()
        .and_then(|rule| rule.branches.first())
        .map(|branch| branch.entry)
        .ok_or(())?;
    let programs = ProgramsEngine {
        code,
        words: base_words(checked(&[]).map_err(|_| ())?, &TEXTS.map(NameText)),
        regexes: REGEXES
            .iter()
            .map(|source| StepRegex((*source).into()))
            .collect(),
        ..ProgramsEngine::default()
    };
    let mut held = held(4);
    // A fault is kept for the rule that was running.
    held.state.rule = Some(Rule(0));
    let (rows, found) = framed(&programs, &mut held, &TheLoop, |frame| {
        run_branch(frame, entry, RunMode::Rows)
    });
    let mut all: Vec<u64> = found.rows.into_iter().flatten().collect();
    all.extend([
        99,
        rows.0,
        held.state.work.unbuilt.0,
        u64::from(held.state.fault.is_some()),
        u64::try_from(held.state.frames.len()).unwrap_or(u64::MAX),
        u64::try_from(held.state.staged.len()).unwrap_or(u64::MAX),
    ]);
    // Then the word asked once more, outside a run, for the first cell: what it leaves staged for its item.
    let first = input.1.first().copied().unwrap_or(0);
    let (asked, _found) = framed(&programs, &mut held, &TheLoop, |frame| {
        let calling = frame
            .fill
            .programs
            .code
            .words
            .iter()
            .copied()
            .find(|word| word.op == Op::Call);
        set(frame, Register(1), CellWord(first));
        if let Some(word) = calling {
            call(frame, word);
        }
        [
            u64::try_from(frame.state.answer.rows.0).unwrap_or(u64::MAX),
            u64::try_from(frame.state.staged.len()).unwrap_or(u64::MAX),
        ]
    });
    all.extend(asked);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
