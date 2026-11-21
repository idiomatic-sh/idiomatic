//! The call contract, one row for each per-row meaning: how many arguments and outputs a call of it has, how many rows it
//! gives for one row, and its cost class; a source search once for each search it may be. Then what the contract holds a
//! call to, and that the traversal says of each call what its shape says.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::steps::calls::fixtures::{ContractCall, contract_calls, searches};
use crate::domain::steps::calls::service::{call_held, call_shape};
use crate::domain::steps::calls::types::{CallForm, SearchGroup, SearchLines, SourceSearch};
use crate::domain::steps::service::{call_class, stage_class, step_class};
use crate::domain::steps::stages::types::FillStage;
use crate::domain::steps::traversal::service::{stage_touched, step_touched};
use crate::domain::steps::types::{CallMeaning, ProgramStep, RegexId, StepOperand};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use std::borrow::Cow;

/// What a row asks of the contract.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The shape and the class of one meaning.
    Shape(CallMeaning),
    /// The shape of a source search, once for each search it may be.
    Searches,
    /// Whether each call of the fixture is held, as written and with one argument more and one output fewer.
    Held,
    /// Whether a search that asks its group's line is held, numbered and not.
    GroupLine,
    /// Whether a `FitsRendered` is held with four, five and six arguments.
    Interleaved,
    /// What the traversal and the class functions say of each call of the fixture, as a step and as a stage.
    Agreement,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// A row of one meaning.
const fn shape(
    name: &'static str,
    meaning: fn() -> Scenario,
    expected: &'static str,
) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input: meaning,
        expect: Expect::Value(Cow::Borrowed(expected)),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    shape(
        "span_between_takes_two_spans_and_gives_one_for_every_row",
        || Scenario::Shape(CallMeaning::SpanBetween),
        "takes Exactly(Count(2)), gives Exactly(Count(1)), rows One, class Constant",
    ),
    shape(
        "span_bounds_takes_a_span_and_gives_its_two_ends_for_every_row",
        || Scenario::Shape(CallMeaning::SpanBounds),
        "takes Exactly(Count(1)), gives Exactly(Count(2)), rows One, class Constant",
    ),
    shape(
        "a_source_search_gives_a_row_for_each_occurrence_and_is_a_fanout",
        || {
            Scenario::Shape(CallMeaning::SourceMatches(
                RegexId(0),
                SourceSearch {
                    lines: SearchLines::Unnumbered,
                    group: SearchGroup::Whole,
                },
            ))
        },
        "takes Exactly(Count(1)), gives Exactly(Count(1)), rows Several, class Fanout",
    ),
    shape(
        "a_source_search_takes_and_gives_what_its_search_says",
        || Scenario::Searches,
        "- 1>1, g 2>2, gt 2>3, gl 2>3, gtl 2>4, n 2>2, ng 3>3, ngt 3>4, ngl 3>4, ngtl 3>5",
    ),
    shape(
        "capture_takes_a_path_and_a_pattern_and_gives_a_row_for_each_way_it_matches",
        || Scenario::Shape(CallMeaning::Capture),
        "takes Exactly(Count(2)), gives AtLeast(Count(0)), rows Several, class Fanout",
    ),
    shape(
        "replace_takes_a_text_and_a_replacement_and_gives_one_text_for_every_row",
        || Scenario::Shape(CallMeaning::Replace(RegexId(0))),
        "takes Exactly(Count(2)), gives Exactly(Count(1)), rows One, class Constant",
    ),
    shape(
        "words_takes_a_text_and_gives_a_row_for_each_word",
        || Scenario::Shape(CallMeaning::Words),
        "takes Exactly(Count(1)), gives Exactly(Count(1)), rows Several, class Fanout",
    ),
    shape(
        "rank_takes_a_text_and_its_layers_and_may_drop_the_row",
        || Scenario::Shape(CallMeaning::Rank),
        "takes AtLeast(Count(1)), gives Exactly(Count(1)), rows AtMostOne, class Constant",
    ),
    shape(
        "concat_takes_any_number_of_texts_and_gives_one_for_every_row",
        || Scenario::Shape(CallMeaning::Concat),
        "takes AtLeast(Count(0)), gives Exactly(Count(1)), rows One, class Constant",
    ),
    shape(
        "lower_takes_a_text_and_gives_one_for_every_row",
        || Scenario::Shape(CallMeaning::Lower),
        "takes Exactly(Count(1)), gives Exactly(Count(1)), rows One, class Constant",
    ),
    shape(
        "fits_rendered_gives_nothing_and_may_drop_the_row",
        || Scenario::Shape(CallMeaning::FitsRendered),
        "takes AtLeast(Count(4)), gives Exactly(Count(0)), rows AtMostOne, class Constant",
    ),
    shape(
        "a_call_is_held_to_its_meanings_counts",
        || Scenario::Held,
        "17 of 17 as written; 2 with an argument more; 1 of 16 with an output fewer",
    ),
    shape(
        "a_groups_line_is_asked_only_of_a_search_that_numbers_lines",
        || Scenario::GroupLine,
        "numbered: Stated; unnumbered: Unstated",
    ),
    shape(
        "fits_rendered_interleaves_its_texts_with_one_literal_more",
        || Scenario::Interleaved,
        "four: Stated; five: Unstated; six: Stated",
    ),
    shape(
        "the_traversal_and_the_classes_say_of_each_call_what_its_shape_says",
        || Scenario::Agreement,
        "17 of 17 calls",
    ),
];

/// A meaning's shape and class, in the table's words.
fn said(meaning: CallMeaning) -> String {
    let shape = call_shape(meaning);
    format!(
        "takes {:?}, gives {:?}, rows {:?}, class {:?}",
        shape.takes,
        shape.gives,
        shape.rows,
        call_class(meaning)
    )
}

/// A search in a few letters: `n` numbered, `g` a group, `t` its text, `l` its line; `-` for none.
fn search_name(search: SourceSearch) -> String {
    let mut name = String::new();
    if search.lines == SearchLines::Numbered {
        name.push('n');
    }
    if let SearchGroup::Named { text, line } = search.group {
        name.push('g');
        if text == TextOutput::Appended {
            name.push('t');
        }
        if line == LineOutput::Appended {
            name.push('l');
        }
    }
    if name.is_empty() {
        name.push('-');
    }
    name
}

/// How many arguments a search takes and outputs it gives, for each search.
fn search_shapes() -> String {
    searches()
        .into_iter()
        .map(|search| {
            let shape = call_shape(CallMeaning::SourceMatches(RegexId(0), search));
            format!(
                "{} {:?}>{:?}",
                search_name(search),
                shape.takes,
                shape.gives
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
        .replace("Exactly(Count(", "")
        .replace("))", "")
}

/// Whether a call is held with its counts changed by `more` arguments and `fewer` outputs.
fn held(call: &ContractCall, more: usize, fewer: usize) -> bool {
    call_held(
        call.meaning,
        Count(call.arguments.0.len().saturating_add(more)),
        Count(call.outputs.0.len().saturating_sub(fewer)),
    ) == CallForm::Stated
}

/// How many of the fixture's calls are held as written; how many are still held with one argument more; and, of those
/// that give a cell, how many are still held with one output fewer.
fn held_counts() -> String {
    let calls = contract_calls();
    let count = |more, fewer| calls.iter().filter(|call| held(call, more, fewer)).count();
    let giving: Vec<&ContractCall> = calls
        .iter()
        .filter(|call| !call.outputs.0.is_empty())
        .collect();
    format!(
        "{} of {} as written; {} with an argument more; {} of {} with an output fewer",
        count(0, 0),
        calls.len(),
        count(1, 0),
        giving.iter().filter(|call| held(call, 0, 1)).count(),
        giving.len()
    )
}

/// What the traversal and the class functions say of each fixture call, as a step and as a stage, against its shape: the
/// reads are its register arguments, the writes its outputs, no body; the step's and the stage's class are the meaning's.
fn agreement() -> String {
    let calls = contract_calls();
    let mut agreed = 0_usize;
    for call in &calls {
        let registers = call
            .arguments
            .0
            .iter()
            .filter(|operand| matches!(operand, StepOperand::Register(_)))
            .count();
        let step = ProgramStep::Call {
            meaning: call.meaning,
            arguments: call.arguments.clone(),
            outputs: call.outputs.clone(),
        };
        let stage = FillStage::Call {
            meaning: call.meaning,
            arguments: call.arguments.clone(),
            outputs: call.outputs.clone(),
        };
        let class = call_class(call.meaning);
        let touched = [step_touched(&step), stage_touched(&stage)];
        let touches = touched.iter().all(|touched| {
            touched.reads.len() == registers
                && touched.writes == *call.outputs.0
                && touched.bodies.is_empty()
        });
        if touches && step_class(&step) == class && stage_class(&stage) == class {
            agreed = agreed.saturating_add(1);
        }
    }
    format!("{agreed} of {} calls", calls.len())
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let search = |lines| SourceSearch {
        lines,
        group: SearchGroup::Named {
            text: TextOutput::Omitted,
            line: LineOutput::Appended,
        },
    };
    let group_line = |lines| {
        let numbered = usize::from(lines == SearchLines::Numbered);
        call_held(
            CallMeaning::SourceMatches(RegexId(0), search(lines)),
            Count(2 + numbered),
            Count(3 + numbered),
        )
    };
    let fits = |arguments| call_held(CallMeaning::FitsRendered, Count(arguments), Count(0));
    Ok(Cow::Owned(match scenario {
        Scenario::Shape(meaning) => said(meaning),
        Scenario::Searches => search_shapes(),
        Scenario::Held => held_counts(),
        Scenario::GroupLine => format!(
            "numbered: {:?}; unnumbered: {:?}",
            group_line(SearchLines::Numbered),
            group_line(SearchLines::Unnumbered)
        ),
        Scenario::Interleaved => format!(
            "four: {:?}; five: {:?}; six: {:?}",
            fits(4),
            fits(5),
            fits(6)
        ),
        Scenario::Agreement => agreement(),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
