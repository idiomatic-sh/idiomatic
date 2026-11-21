//! The constants of the `warning` part: every word of a warning. A form is its lines, with named places the service fills:
//! the first line names the rule and the file; then, one form for each kind of work the engine names, what is wrong (its
//! `{subject}` the statement or the plan step), how much it is, and how to fix it.

use super::types::WarningForm;

/// The first line of the warning of a rule whose work passed its allowance.
pub(crate) const SLOW: WarningForm<'static> =
    WarningForm("warning: rule `{rule}` is slow in `{file}`");

/// The first line of the warning of a rule that read a relation past its declared size.
pub(crate) const OVERSIZED: WarningForm<'static> =
    WarningForm("warning: rule `{rule}` reads a relation past its declared size in `{file}`");

/// A statement, with the line it is written on.
pub(crate) const STATEMENT_AT_LINE: WarningForm<'static> =
    WarningForm("statement `{statement}` ({document}:{line})");

/// A statement whose document the rule's query does not hold: no line is counted.
pub(crate) const STATEMENT: WarningForm<'static> =
    WarningForm("statement `{statement}` ({document})");

/// The plan step, when the query language says no statement.
pub(crate) const PLAN_STEP: WarningForm<'static> = WarningForm("step {step} of the rule's plan");

/// An input of a statement, by the name the statement reads it under.
pub(crate) const NAMED_INPUT: WarningForm<'static> = WarningForm("`{name}`");

/// The two inputs of a statement nobody names, in order.
pub(crate) const UNNAMED_INPUTS: [WarningForm<'static>; 2] = [
    WarningForm("its left input"),
    WarningForm("its right input"),
];

/// What separates a warning's lines: each line after the first is indented under it.
pub(crate) const NEXT_LINE: &str = "\n  ";

/// The rule's own program.
pub(crate) const PROGRAM: [WarningForm<'static>; 3] = [
    WarningForm(
        "{subject} visits {visited} rows from {candidates} candidates in the rule's own program",
    ),
    WarningForm(
        "that is {visited} rows visited in this one file; the run continues and its findings are complete",
    ),
    WarningForm(
        "to fix: start from the rarest thing the rule needs, or reach the rest by a key, so each candidate visits few rows",
    ),
];

/// A definition asked by key.
pub(crate) const CALLS: [WarningForm<'static>; 3] = [
    WarningForm("{subject} is asked for {keys} keys and visits {visited} rows to answer them"),
    WarningForm(
        "that is {visited} rows visited in this one file; the run continues and its findings are complete",
    ),
    WarningForm(
        "to fix: ask it by a column that tells its rows apart, so each key reaches few rows",
    ),
];

/// A join with no key.
pub(crate) const PRODUCT: [WarningForm<'static>; 3] = [
    WarningForm(
        "{subject} joins {left_side} ({left} rows) with {right_side} ({right} rows) with no key between them",
    ),
    WarningForm(
        "that is {pairs} pairs in this one file; the run continues and its findings are complete",
    ),
    WarningForm(
        "to fix: join on a shared column or add a range, so each row meets only its matches",
    ),
];

/// A table of rows read by key.
pub(crate) const ROWS: [WarningForm<'static>; 3] = [
    WarningForm("{subject} reads {read} rows by its keys and makes {made} rows"),
    WarningForm(
        "that is {total} rows read and made in this one file; the run continues and its findings are complete",
    ),
    WarningForm("to fix: join on a column that tells the rows apart, so each key matches few rows"),
];

/// A group.
pub(crate) const GROUP: [WarningForm<'static>; 3] = [
    WarningForm("{subject} groups {read} rows into {made} rows"),
    WarningForm(
        "that is {total} rows read and made in this one file; the run continues and its findings are complete",
    ),
    WarningForm("to fix: keep fewer rows before the group, or group by fewer columns"),
];

/// A sort.
pub(crate) const SORTED: [WarningForm<'static>; 3] = [
    WarningForm("{subject} sorts {rows} rows"),
    WarningForm(
        "that is {rows} rows sorted in this one file; the run continues and its findings are complete",
    ),
    WarningForm("to fix: keep fewer rows before the sort"),
];

/// An ordered match.
pub(crate) const ORDERED_PAIRS: [WarningForm<'static>; 3] = [
    WarningForm(
        "{subject} matches {left_side} ({left} rows) with {right_side} ({right} rows) in order and makes {pairs} pairs",
    ),
    WarningForm(
        "that is {pairs} pairs in this one file; the run continues and its findings are complete",
    ),
    WarningForm(
        "to fix: use `as-of` for the nearest row, or bound the order on both sides, so each row meets only its neighbours",
    ),
];

/// A graph.
pub(crate) const GRAPH: [WarningForm<'static>; 3] = [
    WarningForm("{subject} walks a graph of {read} rows and makes {made} rows"),
    WarningForm(
        "that is {total} rows read and made in this one file; the run continues and its findings are complete",
    ),
    WarningForm(
        "to fix: walk from fewer starts or over fewer edges, so the walk reaches only what the rule reads",
    ),
];

/// A relation past its declared size.
pub(crate) const RELATION: [WarningForm<'static>; 3] = [
    WarningForm("relation `{relation}` holds more rows there than the size it declares allows"),
    WarningForm(
        "the rule's work is {work} rows against an allowance of {allowance} in this one file; the run continues and its findings are complete",
    ),
    WarningForm(
        "to fix: this is the language's to fix: the relation is filled with fewer rows, or declares the size it has",
    ),
];

/// The place of a form that the service fills: `rule`.
pub(crate) const AT_RULE: WarningForm<'static> = WarningForm("{rule}");

/// The place of a form that the service fills: `file`.
pub(crate) const AT_FILE: WarningForm<'static> = WarningForm("{file}");

/// The place of a form that the service fills: `subject`.
pub(crate) const AT_SUBJECT: WarningForm<'static> = WarningForm("{subject}");

/// The place of a form that the service fills: `statement`.
pub(crate) const AT_STATEMENT: WarningForm<'static> = WarningForm("{statement}");

/// The place of a form that the service fills: `document`.
pub(crate) const AT_DOCUMENT: WarningForm<'static> = WarningForm("{document}");

/// The place of a form that the service fills: `line`.
pub(crate) const AT_LINE: WarningForm<'static> = WarningForm("{line}");

/// The place of a form that the service fills: `step`.
pub(crate) const AT_STEP: WarningForm<'static> = WarningForm("{step}");

/// The place of a form that the service fills: `name`.
pub(crate) const AT_NAME: WarningForm<'static> = WarningForm("{name}");

/// The place of a form that the service fills: `left_side`.
pub(crate) const AT_LEFT_SIDE: WarningForm<'static> = WarningForm("{left_side}");

/// The place of a form that the service fills: `right_side`.
pub(crate) const AT_RIGHT_SIDE: WarningForm<'static> = WarningForm("{right_side}");

/// The place of a form that the service fills: `left`.
pub(crate) const AT_LEFT: WarningForm<'static> = WarningForm("{left}");

/// The place of a form that the service fills: `right`.
pub(crate) const AT_RIGHT: WarningForm<'static> = WarningForm("{right}");

/// The place of a form that the service fills: `pairs`.
pub(crate) const AT_PAIRS: WarningForm<'static> = WarningForm("{pairs}");

/// The place of a form that the service fills: `candidates`.
pub(crate) const AT_CANDIDATES: WarningForm<'static> = WarningForm("{candidates}");

/// The place of a form that the service fills: `visited`.
pub(crate) const AT_VISITED: WarningForm<'static> = WarningForm("{visited}");

/// The place of a form that the service fills: `keys`.
pub(crate) const AT_KEYS: WarningForm<'static> = WarningForm("{keys}");

/// The place of a form that the service fills: `read`.
pub(crate) const AT_READ: WarningForm<'static> = WarningForm("{read}");

/// The place of a form that the service fills: `made`.
pub(crate) const AT_MADE: WarningForm<'static> = WarningForm("{made}");

/// The place of a form that the service fills: `total`.
pub(crate) const AT_TOTAL: WarningForm<'static> = WarningForm("{total}");

/// The place of a form that the service fills: `rows`.
pub(crate) const AT_ROWS: WarningForm<'static> = WarningForm("{rows}");

/// The place of a form that the service fills: `relation`.
pub(crate) const AT_RELATION: WarningForm<'static> = WarningForm("{relation}");

/// The place of a form that the service fills: `work`.
pub(crate) const AT_WORK: WarningForm<'static> = WarningForm("{work}");

/// The place of a form that the service fills: `allowance`.
pub(crate) const AT_ALLOWANCE: WarningForm<'static> = WarningForm("{allowance}");
