//! Derivation of ordered stored predicates and physical output maps from a compiled plan.

use crate::domain::stored_selection::types::{PreparedPlan, Selector};
use ryo_relations::domain::relations::service::kinds;
use ryo_values::domain::plan_parts::service::{column_in, parameter_in, step_in};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, Operand, ParameterId, RepeatedRows, Test,
};
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;

/// A plan's static stored selectors; no file population, provider or mutable symbols is accepted.
#[derive(Debug)]
pub(crate) struct SelectionProgram<'a> {
    /// Compiled plan being described, without any physical file population.
    plan: &'a Plan,
    /// Request values borrowed only while preparing immutable descriptors.
    bindings: &'a [ParameterBinding],
}

impl<'a> SelectionProgram<'a> {
    /// Borrow the compiled plan and its frozen request bindings.
    pub(crate) fn new(plan: &'a Plan, bindings: &'a [ParameterBinding]) -> Self {
        Self { plan, bindings }
    }

    /// Compile supported chains in topological order; unsupported steps retain ordinary execution.
    pub(crate) fn prepare(&self) -> PreparedPlan {
        let mut steps: Vec<Option<Arc<Selector>>> = Vec::with_capacity(self.plan.steps.len());
        for step in &self.plan.steps {
            let selector = (step.scope == StepScope::PerFile)
                .then(|| self.derive(&step.operation, &steps))
                .flatten()
                .map(Arc::new);
            steps.push(selector);
        }
        PreparedPlan { steps }
    }

    /// Compose the input map without changing its predicates, bags or physical order.
    fn input(&self, from: Input, steps: &[Option<Arc<Selector>>]) -> Option<Selector> {
        match from {
            Input::BoundObservation(_) => None,
            Input::Step(id) => step_in(steps, id)?
                .as_deref()
                .filter(|selector| !selector.distinct)
                .cloned(),
            // A relation name does not determine its physical representation.
            // The owner admits this alternative only for actual Stored rows.
            Input::Relation(relation) => Some(Selector {
                relation,
                identity: Arc::new(Default::default()),
                columns: (0..kinds(relation).len())
                    .map(|index| ColumnIndex(u32::try_from(index).unwrap_or(u32::MAX)))
                    .collect(),
                distinct: false,
            }),
        }
    }

    /// Derive pure stored chains and terminal distinct maps; later operations keep their own execution.
    fn derive(&self, operation: &Operation, steps: &[Option<Arc<Selector>>]) -> Option<Selector> {
        match operation {
            Operation::Select {
                from,
                comparisons,
                kept,
            } => {
                let mut selector = self.input(*from, steps)?;
                for comparison in comparisons {
                    // These families have no data-dependent refusal. Regex, functions and numeric-text
                    // parsing remain owner-local, so a successful cache never changes their error origin.
                    if !matches!(
                        comparison.test,
                        Test::Equal
                            | Test::NotEqual
                            | Test::NaturalLessThan
                            | Test::LessThan
                            | Test::LessThanOrEqual
                            | Test::GreaterThan
                            | Test::GreaterThanOrEqual
                            | Test::StartsWith
                            | Test::EndsWith
                            | Test::In
                            | Test::NotIn
                    ) {
                        return None;
                    }
                    let mut normalized = comparison.clone();
                    normalized.left = self.operand(&comparison.left, &mut selector)?;
                    normalized.right = self.operand(&comparison.right, &mut selector)?;
                    normalized.origin =
                        ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded;
                    Arc::make_mut(&mut selector.identity)
                        .comparisons
                        .push(normalized);
                }
                if !kept.is_empty() {
                    selector.columns = kept
                        .iter()
                        .map(|index| column_in(&selector.columns, *index).copied())
                        .collect::<Option<_>>()?;
                }
                Some(selector)
            }
            Operation::Project {
                from,
                columns,
                distinct,
            } => {
                let mut selector = self.input(*from, steps)?;
                selector.columns = columns
                    .iter()
                    .map(|index| column_in(&selector.columns, *index).copied())
                    .collect::<Option<_>>()?;
                selector.distinct = *distinct == RepeatedRows::Dropped;
                Some(selector)
            }
            Operation::Extend { .. }
            | Operation::Join { .. }
            | Operation::Union { .. }
            | Operation::Intersection { .. }
            | Operation::Difference { .. }
            | Operation::Antijoin { .. }
            | Operation::GroupCount { .. }
            | Operation::GroupDigest { .. }
            | Operation::Axis { .. }
            | Operation::Capture { .. }
            | Operation::Rank { .. }
            | Operation::Replace { .. }
            | Operation::Words { .. }
            | Operation::SpanBounds { .. }
            | Operation::SourceMatches { .. }
            | Operation::RenderSyntax { .. }
            | Operation::FitsRendered { .. }
            | Operation::SpanBetween { .. }
            | Operation::Closure { .. }
            | Operation::Cycles { .. }
            | Operation::Rows { .. }
            | Operation::OrderedMatch { .. }
            | Operation::OrderedDfs { .. }
            | Operation::Hosted { .. } => None,
        }
    }

    /// Remap physical columns and only the bindings this exact predicate sequence references.
    fn operand(&self, operand: &Operand, selector: &mut Selector) -> Option<Operand> {
        Some(match operand {
            Operand::Column(index) => Operand::Column(*column_in(&selector.columns, *index)?),
            Operand::Literal(value) => Operand::Literal(value.clone()),
            Operand::Parameter(id) => {
                let value = parameter_in(&self.bindings, id)?;
                if matches!(
                    value,
                    ParameterBinding::Tables(_) | ParameterBinding::TextSets(_)
                ) {
                    return None;
                }
                let index = selector
                    .identity
                    .bindings
                    .iter()
                    .position(|bound| bound == value)
                    .unwrap_or_else(|| {
                        let identity = Arc::make_mut(&mut selector.identity);
                        identity.bindings.push(value.clone());
                        identity.bindings.len() - 1
                    });
                let Ok(index) = u32::try_from(index) else {
                    return None;
                };
                Operand::Parameter(ParameterId(index))
            }
            Operand::Function(_) => return None,
        })
    }
}
