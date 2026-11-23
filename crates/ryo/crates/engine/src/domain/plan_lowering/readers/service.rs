//! The `readers` service: multiplicity, column use and aliases, as the prototype's planner reads them.
//!
//! Carried from the prototype for equality with its plans: its walk of an expression's parts skips the first argument of
//! a function and the first item of a list (it walks `e[1:]` of the argument list), so a column read only there is not
//! seen. That is a defect of the prototype's passes, kept here until both are fixed together.

use super::types::{Emitted, PlanReaders, ReadAlias, Worked};
use ryo_values::domain::finish_plans::arguments::groups::types::CountedRows;
use ryo_values::domain::finish_plans::arguments::literals::types::ArgumentValue;
use ryo_values::domain::finish_plans::arguments::texts::types::OptionalColumn;
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::constants::{
    ARG_DOMAIN, ARG_EDGES, ARG_FROM, ARG_LABELS, ARG_LEAVES, ARG_LEFT, ARG_RELATION, ARG_RIGHT,
};
use ryo_values::domain::finish_plans::expressions::types::{PlanConstant, PlanExpression, SourceAlias};
use ryo_values::domain::finish_plans::service::{direction_name, inputs, readers};
use ryo_values::domain::finish_plans::types::{FinishPlan, FinishStep, RuleEmit, SelectStep};
use hashbrown::HashSet;

/// The plan with who reads each step and which are emitted.
pub(crate) fn plan_readers(plan: &FinishPlan) -> PlanReaders<'_> {
    let mut emitted = vec![Emitted::No; plan.steps.len()];
    for rule in &plan.rules {
        if let RuleEmit::Written(emit) = &rule.emit
            && let Some(flag) = emitted.get_mut(emit.step.0)
        {
            *flag = Emitted::Yes;
        }
    }
    PlanReaders {
        steps: &plan.steps,
        readers: readers(&plan.steps),
        emitted,
    }
}

/// The step at a number: every number a plan names is one of its steps.
fn step_at<'s>(plan: &PlanReaders<'s>, number: StepNumber) -> Option<&'s FinishStep> {
    plan.steps.get(number.0)
}

/// The names under which `step` reads `input`: `src` for a select, else each relation argument that names it.
fn roles<'s>(step: &'s FinishStep, input: StepNumber) -> Vec<&'s str> {
    let named = |pairs: &[(&'static str, StepNumber)]| -> Vec<&'s str> {
        pairs
            .iter()
            .filter(|(_, step)| *step == input)
            .map(|(name, _)| *name)
            .collect()
    };
    match step {
        FinishStep::Select(_) => vec![ARG_FROM],
        FinishStep::Union(union) => named(&[(ARG_LEFT, union.left), (ARG_RIGHT, union.right)]),
        FinishStep::Antijoin(antijoin) => {
            named(&[(ARG_LEFT, antijoin.left), (ARG_RIGHT, antijoin.right)])
        }
        FinishStep::AsOf(as_of) => named(&[(ARG_LEFT, as_of.left), (ARG_RIGHT, as_of.right)]),
        FinishStep::OrderedMatch(matched) => {
            named(&[(ARG_LEFT, matched.left), (ARG_RIGHT, matched.right)])
        }
        FinishStep::Axis(axis) => named(&[
            (ARG_FROM, axis.arguments.from),
            (ARG_RELATION, axis.arguments.relation),
            (ARG_DOMAIN, axis.arguments.domain),
        ]),
        FinishStep::Closure(edges) | FinishStep::Follow(edges) => named(&[(ARG_EDGES, edges.edges)]),
        FinishStep::OrderedDfs(walk) => named(&[(ARG_FROM, walk.from), (ARG_EDGES, walk.edges)]),
        FinishStep::Prefixes(prefixes) => named(&[(ARG_FROM, prefixes.from)]),
        FinishStep::SubtreeClass(classes) => named(&[
            (ARG_FROM, classes.from),
            (ARG_RELATION, classes.relation),
            (ARG_LABELS, classes.labels),
            (ARG_LEAVES, classes.leaves),
        ]),
        FinishStep::Unlisted(unlisted) => unlisted
            .arguments
            .iter()
            .filter(|argument| argument.value == ArgumentValue::Step(input))
            .map(|argument| &*argument.name.0)
            .collect(),
        FinishStep::Base(_) | FinishStep::Rows(_) => Vec::new(),
        FinishStep::GroupCount(_)
        | FinishStep::GroupMin(_)
        | FinishStep::GroupMax(_)
        | FinishStep::GroupOrdinal(_)
        | FinishStep::Project(_)
        | FinishStep::Extend(_)
        | FinishStep::SpanBounds(_)
        | FinishStep::SpanBetween(_)
        | FinishStep::Replace(_)
        | FinishStep::SourceMatches(_)
        | FinishStep::Capture(_)
        | FinishStep::FitsRendered(_)
        | FinishStep::RenderSyntax(_) => {
            if inputs(step).contains(&input) {
                vec![ARG_FROM]
            } else {
                Vec::new()
            }
        }
    }
}

/// Whether a step passes each row of its `from` through on its own (`extend`, the span and text steps).
fn row_wise(step: &FinishStep) -> bool {
    matches!(
        step,
        FinishStep::Extend(_)
            | FinishStep::SpanBetween(_)
            | FinishStep::SpanBounds(_)
            | FinishStep::SourceMatches(_)
            | FinishStep::Replace(_)
            | FinishStep::Capture(_)
            | FinishStep::RenderSyntax(_)
            | FinishStep::FitsRendered(_)
    )
}

/// Whether a `group-count` counts distinct values (a `distinct` that is not empty).
fn counts_distinct(step: &FinishStep) -> bool {
    matches!(step, FinishStep::GroupCount(count) if matches!(&count.distinct, CountedRows::Distinct(column) if !column.0.is_empty()))
}

/// Whether no reader of `step` can tell a row of it from its copies: each reader, through steps that pass rows on,
/// ends in an antijoin's right side, a union, a projection, a group ordinal, a distinct count or an emit.
pub(crate) fn blind(plan: &PlanReaders<'_>, memo: &mut [Worked], step: StepNumber) -> bool {
    match memo.get(step.0).copied() {
        Some(Worked::Holds) => return true,
        Some(Worked::Fails) => return false,
        Some(Worked::Open) | None => {}
    }
    let mut holds = true;
    for &reader in plan.readers.get(step.0).map_or(&[][..], Vec::as_slice) {
        let Some(read) = step_at(plan, reader) else {
            continue;
        };
        for role in roles(read, step) {
            let passes = match read {
                FinishStep::Antijoin(_) if role == ARG_RIGHT => true,
                FinishStep::Union(_) | FinishStep::Project(_) | FinishStep::GroupOrdinal(_) => true,
                FinishStep::GroupCount(_) if counts_distinct(read) => true,
                FinishStep::Select(_) => blind(plan, memo, reader),
                FinishStep::Antijoin(_) if role == ARG_LEFT => blind(plan, memo, reader),
                FinishStep::Axis(_) if role == ARG_FROM => blind(plan, memo, reader),
                other if row_wise(other) && role == ARG_FROM => blind(plan, memo, reader),
                _ => false,
            };
            holds &= passes;
        }
    }
    if let Some(slot) = memo.get_mut(step.0) {
        *slot = if holds { Worked::Holds } else { Worked::Fails };
    }
    holds
}

/// Whether any reader of `step`, through steps that pass the column on, reads its column `column`; an emitted step's
/// columns are all read.
pub(crate) fn used(plan: &PlanReaders<'_>, memo: &mut [Worked], step: StepNumber, column: &str) -> bool {
    if plan.emitted.get(step.0) == Some(&Emitted::Yes) {
        return true;
    }
    match memo.get(step.0).copied() {
        Some(Worked::Holds) => return true,
        Some(Worked::Fails) => return false,
        Some(Worked::Open) | None => {}
    }
    let mut read = false;
    for &reader in plan.readers.get(step.0).map_or(&[][..], Vec::as_slice) {
        let Some(reading) = step_at(plan, reader) else {
            continue;
        };
        read = match reading {
            FinishStep::Select(select) => selects_column(plan, memo, select, step, reader, column),
            FinishStep::Antijoin(antijoin) => {
                (antijoin.left == step
                    && (antijoin.left_on.iter().any(|on| &*on.0 == column)
                        || used(plan, memo, reader, column)))
                    || (antijoin.right == step && antijoin.right_on.iter().any(|on| &*on.0 == column))
            }
            FinishStep::Axis(axis) if axis.arguments.from == step => {
                mentions(reading, column) || column == "id" || used(plan, memo, reader, column)
            }
            other if row_wise(other) => mentions(other, column) || used(plan, memo, reader, column),
            FinishStep::GroupCount(_) | FinishStep::GroupOrdinal(_) => mentions(reading, column),
            _ => true,
        };
        if read {
            break;
        }
    }
    if let Some(slot) = memo.get_mut(step.0) {
        *slot = if read { Worked::Holds } else { Worked::Fails };
    }
    read
}

/// Whether a select reads `column` of its source `step`: in its condition or items, or, for `*`, through its readers.
fn selects_column(
    plan: &PlanReaders<'_>,
    memo: &mut [Worked],
    select: &SelectStep,
    step: StepNumber,
    reader: StepNumber,
    column: &str,
) -> bool {
    let aliases: Vec<ReadAlias> = select
        .sources
        .iter()
        .filter(|source| source.step == step)
        .map(|source| read_alias(&source.alias))
        .collect();
    let single = select.sources.len() == 1;
    let reads = |alias: &ReadAlias, name: &PlanConstant| {
        matches!(name, PlanConstant::Text(text) if &*text.0 == column)
            && (aliases.contains(alias) || (*alias == ReadAlias::Unaliased && single))
    };
    let in_condition = match &select.condition {
        ryo_values::domain::finish_plans::types::RowCondition::Every => false,
        ryo_values::domain::finish_plans::types::RowCondition::Tested(condition) => {
            any_column(condition, &reads)
        }
    };
    match &select.columns {
        ryo_values::domain::finish_plans::types::SelectedColumns::All => {
            in_condition || used(plan, memo, reader, column)
        }
        ryo_values::domain::finish_plans::types::SelectedColumns::Items(items) => {
            in_condition || items.iter().any(|item| any_column(&item.value, &reads))
        }
    }
}

/// A plan alias as the passes compare it.
fn read_alias(alias: &SourceAlias) -> ReadAlias {
    match alias {
        SourceAlias::Unwritten => ReadAlias::Unaliased,
        SourceAlias::Written(name) => ReadAlias::Named(name.0.clone()),
    }
}

/// A constant in an alias's place, as the passes compare it.
fn constant_alias(constant: &PlanConstant) -> ReadAlias {
    match constant {
        PlanConstant::Absent => ReadAlias::Unaliased,
        PlanConstant::Text(text) => ReadAlias::Named(text.0.clone()),
        other @ (PlanConstant::Number(_) | PlanConstant::List(_) | PlanConstant::Map(_)) => {
            ReadAlias::Other(other.clone())
        }
    }
}

/// Whether `reads` holds of a column the expression reads (its alias and its name), walked as the prototype walks it.
fn any_column(expression: &PlanExpression, reads: &dyn Fn(&ReadAlias, &PlanConstant) -> bool) -> bool {
    let mut found = false;
    columns_read(expression, &mut |alias, name| found |= reads(alias, name));
    found
}

/// Every column the expression reads, as (alias, name), walked as the prototype walks it: a function's first argument
/// and a list's first item are not walked, and a constant list `["col", name, alias, ..]` reads a column.
pub(crate) fn columns_read(expression: &PlanExpression, seen: &mut dyn FnMut(&ReadAlias, &PlanConstant)) {
    match expression {
        PlanExpression::Column(read) => seen(
            &read_alias(&read.alias),
            &PlanConstant::Text(ryo_values::domain::plan_parts::types::LiteralText(
                read.column.0.clone(),
            )),
        ),
        PlanExpression::Literal(constant) => constant_columns(constant, seen),
        PlanExpression::Number(_) => {}
        PlanExpression::List(items) => items.iter().skip(1).for_each(|item| columns_read(item, seen)),
        PlanExpression::Call(call) => call
            .arguments
            .iter()
            .skip(1)
            .for_each(|argument| columns_read(argument, seen)),
        PlanExpression::Compare(comparison) => {
            columns_read(&comparison.left, seen);
            columns_read(&comparison.right, seen);
        }
        PlanExpression::In(pair)
        | PlanExpression::NotIn(pair)
        | PlanExpression::And(pair)
        | PlanExpression::Or(pair) => {
            columns_read(&pair.left, seen);
            columns_read(&pair.right, seen);
        }
        PlanExpression::Not(inner) => columns_read(inner, seen),
    }
}

/// The columns a constant reads as the prototype's walk reads it: a list whose first item is the text `col` reads the
/// column its second item names under its third; any other list is walked from its second item.
fn constant_columns(constant: &PlanConstant, seen: &mut dyn FnMut(&ReadAlias, &PlanConstant)) {
    let PlanConstant::List(items) = constant else {
        return;
    };
    match items.first() {
        None => {}
        Some(PlanConstant::Text(head)) if &*head.0 == "col" => {
            if let (Some(name), Some(alias)) = (items.get(1), items.get(2)) {
                seen(&constant_alias(alias), name);
            }
        }
        Some(_) => items.iter().skip(1).for_each(|item| constant_columns(item, seen)),
    }
}

/// The aliases an expression reads columns under.
pub(crate) fn aliases_read(expression: &PlanExpression) -> HashSet<ReadAlias> {
    let mut found = HashSet::new();
    columns_read(expression, &mut |alias, _| {
        let _new = found.insert(alias.clone());
    });
    found
}

/// The conditions an `and` tree joins, left to right.
pub(crate) fn conjuncts(expression: &PlanExpression) -> Vec<&PlanExpression> {
    match expression {
        PlanExpression::And(pair) => {
            let mut all = conjuncts(&pair.left);
            all.extend(conjuncts(&pair.right));
            all
        }
        other @ (PlanExpression::Column(_)
        | PlanExpression::Literal(_)
        | PlanExpression::Number(_)
        | PlanExpression::List(_)
        | PlanExpression::Call(_)
        | PlanExpression::Compare(_)
        | PlanExpression::In(_)
        | PlanExpression::NotIn(_)
        | PlanExpression::Or(_)
        | PlanExpression::Not(_)) => vec![other],
    }
}

/// The plan's alias of a source, as the passes compare it.
pub(crate) fn source_alias(alias: &SourceAlias) -> ReadAlias {
    read_alias(alias)
}

/// Whether a text argument of a step (any text its arguments hold, a map's values but not its keys) is `column`.
fn mentions(step: &FinishStep, column: &str) -> bool {
    let is = |text: &str| text == column;
    let optional = |written: &OptionalColumn| matches!(written, OptionalColumn::Written(name) if is(&name.0));
    match step {
        FinishStep::Extend(extend) => extend.values.iter().any(|value| constant_mentions(&value.value, column)),
        FinishStep::SpanBetween(between) => [&between.start, &between.end, &between.output].iter().any(|name| is(&name.0)),
        FinishStep::SpanBounds(bounds) => {
            [&bounds.span, &bounds.start_output, &bounds.end_output].iter().any(|name| is(&name.0))
        }
        FinishStep::SourceMatches(matches) => {
            is(&matches.span.0)
                || is(&matches.pattern.0)
                || is(&matches.output.0)
                || [
                    &matches.base_line,
                    &matches.line_output,
                    &matches.capture,
                    &matches.capture_output,
                    &matches.capture_text_output,
                    &matches.capture_line_output,
                ]
                .into_iter()
                .any(optional)
        }
        FinishStep::Replace(replace) => {
            is(&replace.text.0) || is(&replace.pattern.0) || is(&replace.replacement.0) || is(&replace.output.0)
        }
        FinishStep::Capture(capture) => {
            is(&capture.path.0) || is(&capture.pattern.0) || capture.captures.iter().any(|name| is(&name.0))
        }
        FinishStep::RenderSyntax(render) => is(&render.node.0) || is(&render.context.0) || is(&render.output.0),
        FinishStep::FitsRendered(fits) => {
            fits.texts.iter().any(|name| is(&name.0))
                || fits.literals.iter().any(|text| is(&text.0))
                || is(&fits.span.0)
        }
        FinishStep::Axis(axis) => is(direction_name(axis.arguments.direction)),
        FinishStep::GroupCount(count) => {
            count.keys.iter().any(|name| is(&name.0))
                || is(&count.count.0)
                || matches!(&count.distinct, CountedRows::Distinct(name) if is(&name.0))
        }
        FinishStep::GroupOrdinal(ordinal) => {
            ordinal.keys.iter().any(|name| is(&name.0)) || is(&ordinal.order.0) || is(&ordinal.output.0)
        }
        FinishStep::Base(_)
        | FinishStep::Select(_)
        | FinishStep::Union(_)
        | FinishStep::Antijoin(_)
        | FinishStep::AsOf(_)
        | FinishStep::OrderedMatch(_)
        | FinishStep::Closure(_)
        | FinishStep::Follow(_)
        | FinishStep::OrderedDfs(_)
        | FinishStep::GroupMin(_)
        | FinishStep::GroupMax(_)
        | FinishStep::Project(_)
        | FinishStep::Rows(_)
        | FinishStep::Prefixes(_)
        | FinishStep::SubtreeClass(_)
        | FinishStep::Unlisted(_) => false,
    }
}

/// Whether a constant holds the text `column`, in a list or as a map's value.
fn constant_mentions(constant: &PlanConstant, column: &str) -> bool {
    match constant {
        PlanConstant::Text(text) => &*text.0 == column,
        PlanConstant::List(items) => items.iter().any(|item| constant_mentions(item, column)),
        PlanConstant::Map(entries) => entries.iter().any(|entry| constant_mentions(&entry.value, column)),
        PlanConstant::Number(_) | PlanConstant::Absent => false,
    }
}
