//! The services of the `calls` part: the shape of a call of each per-row meaning, by a `match` that names every meaning (a
//! new meaning does not compile without its row), and whether a call as written has that shape.

use super::types::{Arity, CallForm, CallRows, CallShape, SearchGroup, SearchLines, SourceSearch};
use crate::domain::steps::types::CallMeaning;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};

/// How many cells a numbered search reads and gives beside an unnumbered one: the base line in, the occurrence's line out.
const fn line_cells(lines: SearchLines) -> Count {
    match lines {
        SearchLines::Unnumbered => Count(0),
        SearchLines::Numbered => Count(1),
    }
}

/// How many cells a search gives of its group: its span, then its text and its line as asked.
const fn group_cells(group: SearchGroup) -> Count {
    match group {
        SearchGroup::Whole => Count(0),
        SearchGroup::Named { text, line } => {
            let text: usize = match text {
                TextOutput::Appended => 1,
                TextOutput::Omitted => 0,
            };
            let line: usize = match line {
                LineOutput::Appended => 1,
                LineOutput::Omitted => 0,
            };
            Count(text.saturating_add(line).saturating_add(1))
        }
    }
}

/// How many arguments a search's group is: its name, when one is asked.
const fn group_name_cells(group: SearchGroup) -> Count {
    match group {
        SearchGroup::Whole => Count(0),
        SearchGroup::Named { .. } => Count(1),
    }
}

/// One cell, then the cells of two counts.
const fn one_and(first: Count, second: Count) -> Count {
    Count(first.0.saturating_add(second.0).saturating_add(1))
}

/// The shape of a call of `meaning`: the one statement of what each meaning takes, gives and how many rows it gives; the
/// table is `CallShape`'s.
#[must_use]
pub(crate) const fn call_shape(meaning: CallMeaning) -> CallShape {
    let (takes, gives, rows) = match meaning {
        CallMeaning::SpanBetween => (
            Arity::Exactly(Count(2)),
            Arity::Exactly(Count(1)),
            CallRows::One,
        ),
        CallMeaning::SpanBounds => (
            Arity::Exactly(Count(1)),
            Arity::Exactly(Count(2)),
            CallRows::One,
        ),
        CallMeaning::SourceMatches(_, search) => (
            Arity::Exactly(one_and(
                line_cells(search.lines),
                group_name_cells(search.group),
            )),
            Arity::Exactly(one_and(line_cells(search.lines), group_cells(search.group))),
            CallRows::Several,
        ),
        CallMeaning::Capture => (
            Arity::Exactly(Count(2)),
            Arity::AtLeast(Count(0)),
            CallRows::Several,
        ),
        CallMeaning::Replace(_) => (
            Arity::Exactly(Count(2)),
            Arity::Exactly(Count(1)),
            CallRows::One,
        ),
        CallMeaning::Words => (
            Arity::Exactly(Count(1)),
            Arity::Exactly(Count(1)),
            CallRows::Several,
        ),
        CallMeaning::Rank => (
            Arity::AtLeast(Count(1)),
            Arity::Exactly(Count(1)),
            CallRows::AtMostOne,
        ),
        CallMeaning::Concat => (
            Arity::AtLeast(Count(0)),
            Arity::Exactly(Count(1)),
            CallRows::One,
        ),
        CallMeaning::Lower => (
            Arity::Exactly(Count(1)),
            Arity::Exactly(Count(1)),
            CallRows::One,
        ),
        CallMeaning::FitsRendered => (
            Arity::AtLeast(Count(4)),
            Arity::Exactly(Count(0)),
            CallRows::AtMostOne,
        ),
    };
    CallShape { takes, gives, rows }
}

/// Whether a list of `count` cells is of the arity.
const fn admits(arity: Arity, count: Count) -> CallForm {
    let admitted = match arity {
        Arity::Exactly(exact) => count.0 == exact.0,
        Arity::AtLeast(least) => count.0 >= least.0,
    };
    if admitted {
        CallForm::Stated
    } else {
        CallForm::Unstated
    }
}

/// Whether a search asks only what it can give: a group's line is counted from the base line, so it is asked only of a
/// search that numbers lines.
const fn searchable(search: SourceSearch) -> CallForm {
    match (search.lines, search.group) {
        (
            SearchLines::Unnumbered,
            SearchGroup::Named {
                text: TextOutput::Appended | TextOutput::Omitted,
                line: LineOutput::Appended,
            },
        ) => CallForm::Unstated,
        (
            SearchLines::Unnumbered,
            SearchGroup::Whole
            | SearchGroup::Named {
                text: TextOutput::Appended | TextOutput::Omitted,
                line: LineOutput::Omitted,
            },
        )
        | (SearchLines::Numbered, SearchGroup::Whole | SearchGroup::Named { .. }) => {
            CallForm::Stated
        }
    }
}

/// Whether a call of `meaning` with `arguments` arguments and `outputs` outputs is one the contract states: the counts are
/// its shape's, a `FitsRendered` interleaves n texts with n + 1 literals (so its arguments are 2n + 4), and a search asks
/// only what it can give. The encoding writes no other call.
#[must_use]
pub(crate) const fn call_held(meaning: CallMeaning, arguments: Count, outputs: Count) -> CallForm {
    let shape = call_shape(meaning);
    match (admits(shape.takes, arguments), admits(shape.gives, outputs)) {
        (CallForm::Stated, CallForm::Stated) => {}
        (CallForm::Unstated, CallForm::Stated | CallForm::Unstated)
        | (CallForm::Stated, CallForm::Unstated) => return CallForm::Unstated,
    }
    match meaning {
        CallMeaning::SourceMatches(_, search) => searchable(search),
        CallMeaning::FitsRendered => {
            if arguments.0.is_multiple_of(2) {
                CallForm::Stated
            } else {
                CallForm::Unstated
            }
        }
        CallMeaning::SpanBetween
        | CallMeaning::SpanBounds
        | CallMeaning::Capture
        | CallMeaning::Replace(_)
        | CallMeaning::Words
        | CallMeaning::Rank
        | CallMeaning::Concat
        | CallMeaning::Lower => CallForm::Stated,
    }
}
