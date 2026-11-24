//! Intern separately bound, typed steps in the same numbering space as a lowered pack.

use super::numbering::service::interned;
use ryo_values::domain::finish_plans::{
    arguments::{literals::types::ArgumentValue, types::StepNumber},
    types::FinishStep,
};

/// The pack lowerer's collision-checked step index for already prepared inputs.
#[derive(Debug, Default)]
pub struct SharedSteps {
    steps: Vec<FinishStep>,
    numbers: hashbrown::HashTable<(u64, StepNumber)>,
    hasher: hashbrown::DefaultHashBuilder,
}

impl SharedSteps {
    /// Add one step after its dependencies, mapping its program-local inputs to this reader.
    pub fn append(&mut self, mut step: FinishStep, inputs: &[usize]) -> usize {
        let map = |s: &mut StepNumber| s.0 = inputs[s.0];
        match &mut step {
            FinishStep::Base(_) | FinishStep::Rows(_) => {}
            FinishStep::Select(s) => {
                for source in &mut s.sources {
                    map(&mut source.step);
                }
            }
            FinishStep::Union(s) => {
                map(&mut s.left);
                map(&mut s.right);
            }
            FinishStep::Antijoin(s) => {
                map(&mut s.left);
                map(&mut s.right);
            }
            FinishStep::AsOf(s) => {
                map(&mut s.left);
                map(&mut s.right);
            }
            FinishStep::OrderedMatch(s) => {
                map(&mut s.left);
                map(&mut s.right);
            }
            FinishStep::Axis(s) => {
                map(&mut s.arguments.from);
                map(&mut s.arguments.relation);
                map(&mut s.arguments.domain);
            }
            FinishStep::Closure(s) => {
                map(&mut s.edges);
            }
            FinishStep::Follow(s) => {
                map(&mut s.edges);
            }
            FinishStep::OrderedDfs(s) => {
                map(&mut s.from);
                map(&mut s.edges);
            }
            FinishStep::GroupCount(s) => {
                map(&mut s.from);
            }
            FinishStep::GroupMin(s) => {
                map(&mut s.from);
            }
            FinishStep::GroupMax(s) => {
                map(&mut s.from);
            }
            FinishStep::GroupOrdinal(s) => {
                map(&mut s.from);
            }
            FinishStep::Project(s) => {
                map(&mut s.from);
            }
            FinishStep::Extend(s) => {
                map(&mut s.from);
            }
            FinishStep::SpanBounds(s) => {
                map(&mut s.from);
            }
            FinishStep::SpanBetween(s) => {
                map(&mut s.from);
            }
            FinishStep::Replace(s) => {
                map(&mut s.from);
            }
            FinishStep::SourceMatches(s) => {
                map(&mut s.from);
            }
            FinishStep::Capture(s) => {
                map(&mut s.from);
            }
            FinishStep::FitsRendered(s) => {
                map(&mut s.from);
            }
            FinishStep::RenderSyntax(s) => {
                map(&mut s.from);
            }
            FinishStep::Prefixes(s) => {
                map(&mut s.from);
            }
            FinishStep::SubtreeClass(s) => {
                map(&mut s.from);
                map(&mut s.relation);
                map(&mut s.labels);
                map(&mut s.leaves);
            }
            FinishStep::Unlisted(s) => {
                for argument in &mut s.arguments {
                    if let ArgumentValue::Step(input) = &mut argument.value {
                        map(input);
                    }
                }
            }
        }
        interned(&mut self.steps, &mut self.numbers, &self.hasher, step).0
    }

    /// Finish building the shared step sequence.
    #[must_use]
    pub fn finish(self) -> Vec<FinishStep> {
        self.steps
    }
}
