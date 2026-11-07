//! The relation-catalog corpus: every catalog relation under every head shape.
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::{
    executable_program::value::ExecutableProgram, PartitionRequirement, ProducerPartitions,
};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::service::ALL;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, PlanComparison, HostedNumber, Input, Operand, ParameterId, RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};

use super::plan;

struct FileSources;
impl ProducerPartitions for FileSources {
    fn hosted(
        &self,
        _: RuleIndex,
        _: HostedNumber,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
    fn parameter(
        &self,
        _: RuleIndex,
        _: ParameterId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::Unbound)
    }
    fn relation(&self, _: Relation) -> PartitionRequirement {
        PartitionRequirement::CurrentFile
    }
    fn observation(
        &self,
        _: RuleIndex,
        _: ryo_values::domain::plan_parts::types::ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
}
pub(in super::super) struct CompiledCorpus {
    pub(in super::super) program: ExecutableProgram,
    pub(in super::super) walk: crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk,
}
pub(in super::super) fn compiled() -> CompiledCorpus {
    let mut plans = Vec::new();
    for &relation in ALL {
        let source = Input::Relation(relation);
        plans.push(plan(
            Operation::Select {
                from: source,
                comparisons: vec![],
                kept: vec![],
            },
            StepScope::PerFile,
        ));
        plans.push(plan(
            Operation::Project {
                from: source,
                columns: vec![ColumnIndex(0), ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            StepScope::PerFile,
        ));
        plans.push(plan(
            Operation::Project {
                from: source,
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            },
            StepScope::PerFile,
        ));
        let mut dependent = plan(
            Operation::Select {
                from: source,
                comparisons: vec![],
                kept: vec![],
            },
            StepScope::PerFile,
        );
        dependent.steps.push(PlanStep {
            operation: Operation::Select {
                from: Input::Step(StepId(0)),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
            scope: StepScope::PerFile,
        });
        dependent.emit.step = StepId(1);
        plans.push(dependent);
        let mut continuation = plan(
            Operation::Select {
                from: source,
                comparisons: vec![],
                kept: vec![],
            },
            StepScope::PerFile,
        );
        if let Operation::Select { comparisons, .. } = &mut continuation.steps[0].operation {
            comparisons.push(PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Column(ColumnIndex(0)),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            });
        }
        continuation.steps.push(PlanStep {
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            scope: StepScope::PerFile,
        });
        continuation.emit.step = StepId(1);
        plans.push(continuation);
        let invalid = ColumnIndex(
            u32::try_from(ryo_relations::domain::relations::service::columns(relation).len())
                .unwrap(),
        );
        plans.push(plan(
            Operation::Project {
                from: source,
                columns: vec![invalid],
                distinct: RepeatedRows::Kept,
            },
            StepScope::PerFile,
        ));
        plans.push(plan(
            Operation::Select {
                from: source,
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(invalid),
                    right: Operand::Column(ColumnIndex(0)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![],
            },
            StepScope::PerFile,
        ));
        let mut project = plan(
            Operation::Select {
                from: source,
                comparisons: vec![],
                kept: vec![],
            },
            StepScope::CrossFile,
        );
        project.frontier.push(source);
        plans.push(project);
    }
    let operated = plans.clone();
    let (program, mut walk) = prepare(&plans, &operated);
    walk.retain_program_consumers(&program).unwrap();
    CompiledCorpus { program, walk }
}
/// Compile `plans` against operator contracts prepared for `operated`, unretained. A plan
/// with steps beyond its operated contracts has unmapped origins.
pub(in super::super) fn prepare(
    plans: &[ryo_values::domain::plans::types::Plan],
    operated: &[ryo_values::domain::plans::types::Plan],
) -> (ExecutableProgram, crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk) {
    let bindings = vec![vec![]; plans.len()];
    let operators = crate::application::file_dag::service::prepare_with_schemas(
        operated,
        &bindings,
        &[],
        &ryo_relations::domain::relations::service::kinds,
        crate::application::file_dag::fixtures::real_derived_port(),
    )
    .unwrap();
    let graph = crate::domain::file_program::service::compile_prepared(plans, &operators);
    let program =
        crate::domain::executable_program::service::compile(plans, graph, &FileSources, &operators, crate::domain::executable_program::types::RoutePolicy::default())
            .unwrap();
    let walk = crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk::compile(plans, &bindings);
    (program, walk)
}
