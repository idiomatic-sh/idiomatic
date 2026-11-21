//! The types of the `calls` part of the `steps` concept: the shape of a call of a per-row meaning, and what a source search
//! reads and gives. Crate-private.

use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};

/// The call contract: what a `Call` step or stage of each meaning takes and gives, as `calls::service::call_shape` answers
/// it and the encoding holds it.
///
/// `arguments` are every cell the meaning reads, in this order: the cells of the row (a register, or a constant where the
/// lowering knows the cell), in the order the operation declares its columns; then the operation's constants, in the order
/// it declares them. A constant text is a base word. A regular expression is not an argument: it is the meaning's own
/// `RegexId`. `outputs` are the cells the meaning gives, in the order today's engine appends them to the row
/// (`execution_domain`'s `operation_schema::service::schema_of`). Nothing else is read or written: what the traversal says a
/// call touches is its register arguments and its outputs.
///
/// | meaning | arguments | outputs | rows for one row | class |
/// |---|---|---|---|---|
/// | `SpanBetween` | start span, end span | the span from the first's start to the second's end; the absent span when either is absent | one | `Constant` |
/// | `SpanBounds` | span | start, end: two numbers | one | `Constant` |
/// | `SourceMatches(regex, search)` | span; the line of the span's first byte, when `search.lines` is `Numbered`; then the group's name as a constant text, when `search.group` is `Named` | the occurrence's span; its line, when numbered; then for a named group its span, its text when `text` is `Appended`, its line when `line` is `Appended` | several: one for each occurrence of the expression in the span's text, left to right; none for an absent span; none for an occurrence whose group took no part | `Fanout` |
/// | `Capture` | path text; the pattern as a constant text | one text for each slot of the pattern, in slot order | several: one for each way the pattern matches the whole path; none for a cell that is no text | `Fanout` |
/// | `Replace(regex)` | text; the replacement as a constant text | the text with every occurrence replaced | one | `Constant` |
/// | `Words` | text | one word, lowercased | several: one for each word, in the text's order, a repeated word repeated; none for a cell that is no text | `Fanout` |
/// | `Rank` | text; then each layer as a constant set, in the layers' order | the place of the first layer that holds the text: a number, the first 0 | at most one: none when no layer holds the text or the cell is no text | `Constant` |
/// | `Concat` | the texts, in order, each a register or a constant | the texts one after the other | one | `Constant` |
/// | `Lower` | text | the text in lowercase | one | `Constant` |
/// | `FitsRendered` | the n text cells, in order; the span; then the n + 1 literals as constant texts, the widest line and the tab stop as constant numbers | none | at most one: the row goes on when the candidate fits | `Constant` |
///
/// How many outputs a `Capture` has is the slot count of its pattern, and the lowering reads it where today's engine does:
/// `ryo_operators::domain::capture::service::slot_count(SlotPattern(&pattern.0))`. A pattern of no slot gives nothing and
/// keeps the row when the pattern is the path.
///
/// Faults of today's meanings, each of the row that raises it (how the engine hands one on is not in this contract):
/// `SpanBetween` on a span out of order or past the file, and on a first span that starts after the second ends;
/// `SpanBounds` on an absent span or one out of order; `SourceMatches` on a span out of order, past the file or off a
/// character boundary, on a base line below 1, and on a group the expression does not name; `Replace` on a cell that is no
/// text; `FitsRendered` on an absent span, one out of order or past the file, on a text cell that is no text, on a widest
/// line below 0 and on a tab stop below 1. `Concat` and `Lower` of a cell that is no text give a cell that is no text and
/// that no text equals: which cell that is in this engine is to be confirmed by a run.
///
/// The class is the row count's: a meaning that gives at most one row does a fixed amount of work for a row (a regular
/// expression is linear in the text it reads), and one that gives several is bounded by the rows it gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CallShape {
    /// How many arguments a call of the meaning has.
    pub(crate) takes: Arity,
    /// How many outputs it has.
    pub(crate) gives: Arity,
    /// How many rows it gives for one row.
    pub(crate) rows: CallRows,
}

/// How many rows a call gives for one row of its input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallRows {
    /// Exactly one: the row goes on with the outputs.
    One,
    /// One or none: the meaning may drop the row.
    AtMostOne,
    /// Any number: the call is an iteration, and each row it gives goes on with that row's outputs.
    Several,
}

/// Whether a call as written is one the contract states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallForm {
    /// Its arguments and outputs are its meaning's.
    Stated,
    /// They are not: the encoding writes no such call.
    Unstated,
}

/// How many cells a list of a call holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Arity {
    /// This many.
    Exactly(Count),
    /// This many or more.
    AtLeast(Count),
}

/// What a `source-matches` call reads and gives beside each occurrence's span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceSearch {
    /// Whether lines are numbered.
    pub(crate) lines: SearchLines,
    /// Whether a named group is asked of each occurrence, and what of it is given.
    pub(crate) group: SearchGroup,
}

/// Whether a source search numbers lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchLines {
    /// No line is read or given.
    Unnumbered,
    /// The line of the span's first byte is the second argument, and each occurrence's line is given after its span.
    Numbered,
}

/// The named group a source search asks of each occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchGroup {
    /// None: the whole occurrence alone.
    Whole,
    /// The group whose name is the last argument: its span is given, then its text and its line as asked. Its line is
    /// asked only of a search that numbers lines.
    Named {
        /// Whether the group's text is given.
        text: TextOutput,
        /// Whether the line the group starts on is given.
        line: LineOutput,
    },
}
