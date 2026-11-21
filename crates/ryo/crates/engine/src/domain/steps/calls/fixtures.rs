//! What the cases of the call contract share: every per-row meaning, a source search once for each search it may be, each
//! with a call of it as the lowering writes one: its arguments and its outputs.

use super::types::{SearchGroup, SearchLines, SourceSearch};
use crate::domain::steps::types::{
    CallMeaning, RegexId, Register, RegisterList, StepOperand, StepOperands,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};

/// A call of a meaning as the contract states it.
#[derive(Clone, Debug)]
pub(crate) struct ContractCall {
    /// The meaning.
    pub(crate) meaning: CallMeaning,
    /// What it reads.
    pub(crate) arguments: StepOperands,
    /// What it gives.
    pub(crate) outputs: RegisterList,
}

/// The regular expression the fixture's searches and replacement name.
const EXPRESSION: RegexId = RegexId(3);

/// The register of an argument, by its place among the row's cells.
const fn cell(place: u32) -> StepOperand {
    StepOperand::Register(Register(place))
}

/// A constant.
const fn constant(bits: u64) -> StepOperand {
    StepOperand::Word(CellWord(bits))
}

/// A call that gives `gives` cells, in registers from 10 on.
fn call(meaning: CallMeaning, arguments: Vec<StepOperand>, gives: u32) -> ContractCall {
    ContractCall {
        meaning,
        arguments: StepOperands(arguments.into_boxed_slice()),
        outputs: RegisterList((10..10_u32.saturating_add(gives)).map(Register).collect()),
    }
}

/// Every search a source search may be: unnumbered and numbered, each with no group and with a group of each output asked
/// or not.
pub(crate) fn searches() -> Vec<SourceSearch> {
    let outputs = [
        (TextOutput::Omitted, LineOutput::Omitted),
        (TextOutput::Appended, LineOutput::Omitted),
        (TextOutput::Omitted, LineOutput::Appended),
        (TextOutput::Appended, LineOutput::Appended),
    ];
    let mut searches = Vec::new();
    for lines in [SearchLines::Unnumbered, SearchLines::Numbered] {
        searches.push(SourceSearch {
            lines,
            group: SearchGroup::Whole,
        });
        for (text, line) in outputs {
            searches.push(SourceSearch {
                lines,
                group: SearchGroup::Named { text, line },
            });
        }
    }
    searches
}

/// A source search as a call: the span, the base line when numbered, the group's name when one is asked; it gives the
/// occurrence's span, its line when numbered, and of a group its span, then its text and its line as asked.
fn search_call(search: SourceSearch) -> ContractCall {
    let mut arguments = vec![cell(1)];
    let mut gives = 1;
    match search.lines {
        SearchLines::Unnumbered => {}
        SearchLines::Numbered => {
            arguments.push(cell(2));
            gives += 1;
        }
    }
    match search.group {
        SearchGroup::Whole => {}
        SearchGroup::Named { text, line } => {
            arguments.push(constant(7));
            gives += 1;
            if text == TextOutput::Appended {
                gives += 1;
            }
            if line == LineOutput::Appended {
                gives += 1;
            }
        }
    }
    call(
        CallMeaning::SourceMatches(EXPRESSION, search),
        arguments,
        gives,
    )
}

/// Every meaning with a call of it, a source search once for each search the contract states: a `Capture` of two slots, a `Rank` of two layers,
/// a `Concat` of three texts, a `FitsRendered` of two texts between three literals.
pub(crate) fn contract_calls() -> Vec<ContractCall> {
    let layer = |words: &[u64]| StepOperand::Words(words.iter().copied().map(CellWord).collect());
    let mut calls = vec![
        call(CallMeaning::SpanBetween, vec![cell(1), cell(2)], 1),
        call(CallMeaning::SpanBounds, vec![cell(1)], 2),
    ];
    // A group's line is counted from the base line: the contract states no unnumbered search that asks it.
    let stated = |search: &SourceSearch| {
        !matches!(
            (search.lines, search.group),
            (
                SearchLines::Unnumbered,
                SearchGroup::Named {
                    line: LineOutput::Appended,
                    ..
                }
            )
        )
    };
    calls.extend(searches().into_iter().filter(stated).map(search_call));
    calls.extend([
        call(CallMeaning::Capture, vec![cell(1), constant(7)], 2),
        call(
            CallMeaning::Replace(EXPRESSION),
            vec![cell(1), constant(7)],
            1,
        ),
        call(CallMeaning::Words, vec![cell(1)], 1),
        call(
            CallMeaning::Rank,
            vec![cell(1), layer(&[7, 8]), layer(&[9])],
            1,
        ),
        call(CallMeaning::Concat, vec![cell(1), constant(7), cell(2)], 1),
        call(CallMeaning::Lower, vec![cell(1)], 1),
        call(
            CallMeaning::FitsRendered,
            vec![
                cell(1),
                cell(2),
                cell(3),
                constant(7),
                constant(8),
                constant(9),
                constant(88),
                constant(4),
            ],
            0,
        ),
    ]);
    calls
}
