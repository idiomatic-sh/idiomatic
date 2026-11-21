//! The service of the `warning` part: the one function that words the engine's notice of a rule's work in a file. The
//! engine gives counts and no text; every word of a warning is a form of this part's constants, one for each kind of work
//! the engine names, each saying what is wrong, how much it is, and how to fix it.

use super::constants::{
    AT_ALLOWANCE, AT_CANDIDATES, AT_DOCUMENT, AT_FILE, AT_KEYS, AT_LEFT, AT_LEFT_SIDE, AT_LINE,
    AT_MADE, AT_NAME, AT_PAIRS, AT_READ, AT_RELATION, AT_RIGHT, AT_RIGHT_SIDE, AT_ROWS, AT_RULE,
    AT_STATEMENT, AT_STEP, AT_SUBJECT, AT_TOTAL, AT_VISITED, AT_WORK, CALLS, GRAPH, GROUP,
    NAMED_INPUT, NEXT_LINE, ORDERED_PAIRS, OVERSIZED, PLAN_STEP, PRODUCT, PROGRAM, RELATION, ROWS,
    SLOW, SORTED, STATEMENT, STATEMENT_AT_LINE, UNNAMED_INPUTS,
};
use super::types::{NoticedStatement, PlaceValue, SlowRule, WarningForm, WarningText};
use ryo_engine::domain::findings::types::{HeaviestWork, WorkNotice, WorkVerdict};
use ryo_relations::domain::relations::service::name;
use ryo_values::domain::query_tokens::types::StepSpelling;

/// A form with each of its named places filled once, in one pass: a value is never read for a place.
fn filled(form: WarningForm<'_>, values: &[PlaceValue<'_>]) -> WarningText {
    let mut text = String::with_capacity(form.0.len());
    let mut rest = form.0;
    while let Some(open) = rest.find('{') {
        let (before, from) = rest.split_at(open);
        text.push_str(before);
        let place = from
            .find('}')
            .map(|close| from.split_at(close.saturating_add(1)));
        let value = place.and_then(|(place, _)| values.iter().find(|each| each.place.0 == place));
        match (place, value) {
            (Some((_, after)), Some(found)) => {
                text.push_str(&found.value.to_string());
                rest = after;
            }
            (..) => {
                let (brace, after) = from.split_at(1);
                text.push_str(brace);
                rest = after;
            }
        }
    }
    text.push_str(rest);
    WarningText(text.into())
}

/// What a warning calls the statement: its name and where it is written, with its line when the rule's query holds its
/// document (the lines of the document's text up to where the statement starts); or the plan step, when none is said.
fn subject(notice: &WorkNotice, statement: NoticedStatement<'_>) -> WarningText {
    let (written, query) = match statement {
        NoticedStatement::Written(written, query) => (written, query),
        NoticedStatement::PlanStep => {
            let step = PlaceValue {
                place: AT_STEP,
                value: &notice.step.0,
            };
            return filled(PLAN_STEP, &[step]);
        }
    };
    let module = query.modules.get(&written.document);
    let own = (query.root.id == written.document).then_some(&query.root);
    let text = own.or(module).map(|document| document.text.as_bytes());
    let line = text.map(|text| {
        let before = text.get(..written.span.start.0).unwrap_or(text);
        let ended = before.iter().filter(|byte| **byte == b'\n').count();
        ended.saturating_add(1)
    });
    let named = [
        PlaceValue {
            place: AT_STATEMENT,
            value: &&*written.statement,
        },
        PlaceValue {
            place: AT_DOCUMENT,
            value: &written.document.0,
        },
    ];
    match line {
        Some(line) => {
            let [statement, document] = named;
            let line = PlaceValue {
                place: AT_LINE,
                value: &line,
            };
            filled(STATEMENT_AT_LINE, &[statement, document, line])
        }
        None => filled(STATEMENT, &named),
    }
}

/// A value for a named place of a form.
const fn place<'a>(place: WarningForm<'a>, value: &'a dyn std::fmt::Display) -> PlaceValue<'a> {
    PlaceValue { place, value }
}

/// What a warning calls an input of the statement: its name as the statement reads it, or its side.
fn side(input: Option<&StepSpelling>, unnamed: WarningForm<'_>) -> WarningText {
    input.map_or_else(
        || WarningText(unnamed.0.into()),
        |input| {
            let name = PlaceValue {
                place: AT_NAME,
                value: &&**input,
            };
            filled(NAMED_INPUT, &[name])
        },
    )
}

/// The engine's notice of a rule's work in a file as the warning the run prints and lists: four lines, the first naming the
/// rule and the file, then what is wrong (the statement, or the plan step when the query language says no statement, with
/// the engine's counts), how much it is, and how to fix it. The rule was not stopped: the third line says so.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::findings::types::{HeaviestWork, InstructionCount, WorkNotice, WorkVerdict};
/// use ryo_execution_runtime::application::file_scan::warning::service::worded;
/// use ryo_execution_runtime::application::file_scan::warning::types::{NoticedStatement, SlowRule};
/// use ryo_language::domain::files::types::SourcePath;
/// use ryo_values::domain::plan_parts::types::StepId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
/// use ryo_values::domain::runs::types::RuleId;
///
/// let notice = WorkNotice {
///     rule: SelectedRuleOrdinal(0),
///     verdict: WorkVerdict::Over,
///     work: InstructionCount(64_000),
///     allowance: InstructionCount(8_000),
///     step: StepId(2),
///     heaviest: HeaviestWork::Sorted { rows: InstructionCount(64_000) },
/// };
/// let rule = RuleId("rule".into());
/// let slow = SlowRule { rule: &rule, file: SourcePath("b.walk"), statement: NoticedStatement::PlanStep };
/// assert!(worded(&notice, &slow).0.starts_with("warning: rule `rule` is slow in `b.walk`\n  step 2 of the rule's plan sorts 64000 rows\n"));
/// ```
#[must_use]
pub fn worded(notice: &WorkNotice, slow: &SlowRule<'_>) -> WarningText {
    let inputs = match slow.statement {
        NoticedStatement::Written(written, _) => written.inputs.as_slice(),
        NoticedStatement::PlanStep => &[],
    };
    let [left_unnamed, right_unnamed] = UNNAMED_INPUTS;
    let (subject, left_side, right_side) = (
        subject(notice, slow.statement),
        side(inputs.first(), left_unnamed),
        side(inputs.get(1), right_unnamed),
    );
    // A form's three lines, the statement and its sides filled in with the counts of the kind of work the form is of.
    let said = |form: [WarningForm<'_>; 3], counts: &[PlaceValue<'_>]| {
        let mut values = vec![
            place(AT_SUBJECT, &subject.0),
            place(AT_LEFT_SIDE, &left_side.0),
            place(AT_RIGHT_SIDE, &right_side.0),
        ];
        values.extend_from_slice(counts);
        form.map(|line| filled(line, &values).0)
    };
    let (first, lines) = match (notice.verdict, notice.heaviest) {
        (WorkVerdict::Oversized(relation), _) => (
            OVERSIZED,
            said(
                RELATION,
                &[
                    place(AT_RELATION, &name(relation).0),
                    place(AT_WORK, &notice.work.0),
                    place(AT_ALLOWANCE, &notice.allowance.0),
                ],
            ),
        ),
        (
            WorkVerdict::Over,
            HeaviestWork::Program {
                candidates,
                visited,
            },
        ) => (
            SLOW,
            said(
                PROGRAM,
                &[
                    place(AT_CANDIDATES, &candidates.0),
                    place(AT_VISITED, &visited.0),
                ],
            ),
        ),
        (WorkVerdict::Over, HeaviestWork::Calls { keys, visited }) => (
            SLOW,
            said(
                CALLS,
                &[place(AT_KEYS, &keys.0), place(AT_VISITED, &visited.0)],
            ),
        ),
        (WorkVerdict::Over, HeaviestWork::Product { left, right }) => (
            SLOW,
            said(
                PRODUCT,
                &[
                    place(AT_LEFT, &left.0),
                    place(AT_RIGHT, &right.0),
                    place(AT_PAIRS, &left.0.saturating_mul(right.0)),
                ],
            ),
        ),
        (WorkVerdict::Over, HeaviestWork::Rows { read, made }) => (
            SLOW,
            said(
                ROWS,
                &[
                    place(AT_READ, &read.0),
                    place(AT_MADE, &made.0),
                    place(AT_TOTAL, &read.0.saturating_add(made.0)),
                ],
            ),
        ),
        (WorkVerdict::Over, HeaviestWork::Group { read, made }) => (
            SLOW,
            said(
                GROUP,
                &[
                    place(AT_READ, &read.0),
                    place(AT_MADE, &made.0),
                    place(AT_TOTAL, &read.0.saturating_add(made.0)),
                ],
            ),
        ),
        (WorkVerdict::Over, HeaviestWork::Sorted { rows }) => {
            (SLOW, said(SORTED, &[place(AT_ROWS, &rows.0)]))
        }
        (WorkVerdict::Over, HeaviestWork::OrderedPairs { left, right, made }) => (
            SLOW,
            said(
                ORDERED_PAIRS,
                &[
                    place(AT_LEFT, &left.0),
                    place(AT_RIGHT, &right.0),
                    place(AT_PAIRS, &made.0),
                ],
            ),
        ),
        (WorkVerdict::Over, HeaviestWork::Graph { read, made }) => (
            SLOW,
            said(
                GRAPH,
                &[
                    place(AT_READ, &read.0),
                    place(AT_MADE, &made.0),
                    place(AT_TOTAL, &read.0.saturating_add(made.0)),
                ],
            ),
        ),
    };
    let named = [place(AT_RULE, &slow.rule.0), place(AT_FILE, &slow.file.0)];
    let first = filled(first, &named).0;
    let [what, much, fix] = lines;
    WarningText([first, what, much, fix].join(NEXT_LINE).into())
}
