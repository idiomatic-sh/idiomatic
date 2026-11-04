//! Explicitly enabled compiled-prefix evidence; this observes plans and never executes or caches them.
use crate::domain::rules::types::RuleSpec;
use crate::domain::table_model::types::ParameterBinding;
use ryo_relations::domain::relations::{service::kinds, types::Relation};
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plan_parts::{
    service::remapped,
    types::{ColumnIndex, Comparison, Input, RepeatedRows},
};
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};

/// A relation identity, ordered predicates over its original columns, and a composed output projection.
type Prefix = (Relation, Vec<Comparison>, Vec<ColumnIndex>);

/// Follow only bag-preserving relation-rooted filters and projections.
/// Distinct, joins, hosted inputs, and other operators are normalization barriers.
fn prefix(below: &[Option<Prefix>], operation: &Operation) -> Option<Prefix> {
    let (from, comparisons, kept) = match operation {
        Operation::Select {
            from,
            comparisons,
            kept,
        } => (*from, comparisons.as_slice(), kept.as_slice()),
        Operation::Project {
            from,
            columns,
            distinct: RepeatedRows::Kept,
        } => (*from, &[][..], columns.as_slice()),
        Operation::Project {
            distinct: RepeatedRows::Dropped,
            ..
        }
        | Operation::Extend { .. }
        | Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBounds { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBetween { .. } => return None,
    };
    let (relation, mut chosen, columns) = match from {
        Input::BoundObservation(_) => return None,
        Input::Relation(relation) => (
            relation,
            Vec::new(),
            (0..u32::try_from(kinds(relation).len()).ok()?)
                .map(ColumnIndex)
                .collect::<Vec<_>>(),
        ),
        Input::Step(step) => below.get(usize::try_from(step.0).ok()?)?.as_ref()?.clone(),
    };
    for comparison in comparisons {
        let map = |column: ColumnIndex| columns.get(usize::try_from(column.0).ok()?).copied();
        chosen.push(Comparison {
            test: comparison.test,
            left: remapped(&comparison.left, &map)?,
            right: remapped(&comparison.right, &map)?,
            origin: comparison.origin,
        });
    }
    let projection = if matches!(operation, Operation::Select { .. }) && kept.is_empty() {
        columns
    } else {
        kept.iter()
            .map(|column| columns.get(usize::try_from(column.0).ok()?).copied())
            .collect::<Option<Vec<_>>>()?
    };
    Some((relation, chosen, projection))
}

/// Emit typed compile-time observations only under the explicit diagnostic target.
/// Grouping predicates without origin IDs does not make their owners' error provenance interchangeable.
pub(super) fn compiled(
    rules: &[RuleSpec],
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
    hosted: &[HostRelation],
) {
    if tracing::enabled!(target: "ryo::diagnostic::compiled_subplan", tracing::Level::TRACE) {
        subplans(rules, plans, bindings, hosted);
    }
    if !tracing::enabled!(target: "ryo::diagnostic::compiled_prefix", tracing::Level::TRACE) {
        return;
    }
    for (rule_index, ((rule, plan), rule_bindings)) in
        rules.iter().zip(plans).zip(bindings).enumerate()
    {
        let mut below = Vec::with_capacity(plan.steps.len());
        for step in &plan.steps {
            below.push(if step.scope == StepScope::PerFile {
                prefix(&below, &step.operation)
            } else {
                None
            });
        }
        // Exact source bytes are bound by per-document digests and logical identities.
        // These are evidence identities, not executable cache keys.
        let sources = std::iter::once((&rule.query.root.id, &rule.query.root))
            .chain(rule.query.modules.iter())
            .map(|(registered, source)| {
                (
                    registered,
                    &source.id,
                    source.text.len(),
                    blake3::hash(source.text.as_bytes()).to_hex().to_string(),
                )
            })
            .collect::<Vec<_>>();
        tracing::event!(
            target: "ryo::diagnostic::compiled_prefix", tracing::Level::TRACE,
            record_kind = "rule", rule_index, rule = %rule.id.0,
            sources = ?sources, parameters = ?plan.parameters, bindings = ?rule_bindings,
            applicability = ?rule.applicability, include = ?rule.include, exclude = ?rule.exclude,
            steps = plan.steps.len(), observed_prefixes = below.iter().filter(|item| item.is_some()).count(),
            normalization = "ordered predicates and bag-preserving projections only",
        );
        for (step_index, observed) in below.iter().enumerate() {
            let Some((relation, comparisons, projection)) = observed else {
                continue;
            };
            let tests = comparisons
                .iter()
                .map(|comparison| (&comparison.test, &comparison.left, &comparison.right))
                .collect::<Vec<_>>();
            let origins = comparisons
                .iter()
                .map(|comparison| comparison.origin)
                .collect::<Vec<_>>();
            tracing::event!(
                target: "ryo::diagnostic::compiled_prefix", tracing::Level::TRACE,
                record_kind = "prefix", rule_index, rule = %rule.id.0, step_index,
                relation = ?relation, comparisons = ?tests, projection = ?projection,
                origin_ids = ?origins, scope = "per-file",
            );
        }
    }
}

/// Full physical operation evidence is opt-in and remains separate from executable cache keys.
/// Source origins and bound values are logged apart from operation shape so an offline census can
/// decide whether a repeated shape is semantically interchangeable.
fn subplans(
    rules: &[RuleSpec],
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
    hosted: &[HostRelation],
) {
    for (rule_index, ((rule, plan), bound)) in rules.iter().zip(plans).zip(bindings).enumerate() {
        let identity = std::ptr::from_ref(plan).addr();
        let sources = std::iter::once((&rule.query.root.id, &rule.query.root))
            .chain(rule.query.modules.iter())
            .map(|(registered, source)| {
                (
                    registered,
                    &source.id,
                    source.text.len(),
                    blake3::hash(source.text.as_bytes()).to_hex().to_string(),
                )
            })
            .collect::<Vec<_>>();
        tracing::event!(
            target: "ryo::diagnostic::compiled_subplan", tracing::Level::TRACE,
            record_kind = "plan", rule_index, rule = %rule.id.0, plan_identity = identity,
            sources = ?sources, parameters = ?plan.parameters, bindings = ?bound,
            hosted = ?hosted, comparison_origins = ?plan.comparison_origins,
            emit = ?plan.emit, frontier = ?plan.frontier, steps = plan.steps.len(),
        );
        for (step_index, step) in plan.steps.iter().enumerate() {
            tracing::event!(
                target: "ryo::diagnostic::compiled_subplan", tracing::Level::TRACE,
                record_kind = "step", rule_index, rule = %rule.id.0, plan_identity = identity,
                step_index, scope = ?step.scope, operation = ?step.operation,
            );
        }
    }
}

#[cfg(test)]
mod tests;
