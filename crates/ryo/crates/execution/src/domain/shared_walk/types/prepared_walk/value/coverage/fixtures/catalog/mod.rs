//! The relation-catalog corpus: every catalog relation under every head shape.
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::{
    ExecutableProgram, PartitionRequirement, ProducerPartitions,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::RequestOperators;
use ryo_relations::domain::relations::service::ALL;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, HostedNumber, Input, Operand, ParameterId, RepeatedRows, StepId, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep, StepScope};

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
        _: ryo_values::domain::observations::types::ObservationBindingId,
    ) -> Result<PartitionRequirement, OperatingError> {
        Err(OperatingError::NotRead)
    }
}
pub(in super::super) struct CompiledCorpus {
    pub(in super::super) program: ExecutableProgram,
    pub(in super::super) walk: super::super::super::PreparedWalk,
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
            comparisons.push(Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Column(ColumnIndex(0)),
                origin: None,
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
                comparisons: vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(invalid),
                    right: Operand::Column(ColumnIndex(0)),
                    origin: None,
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
    let bindings = vec![vec![]; plans.len()];
    let operators = RequestOperators::prepare_with_schemas(
        &plans,
        &bindings,
        &[],
        &ryo_relations::domain::relations::service::kinds,
        std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ),
    )
    .unwrap();
    let graph = crate::domain::file_program::service::compile_prepared(&plans, &operators);
    let program = crate::domain::executable_program::service::compile(
        &plans,
        graph,
        &FileSources,
        &operators,
    )
    .unwrap();
    let mut walk = super::super::super::PreparedWalk::compile(&plans, &bindings);
    walk.retain_program_consumers(&program).unwrap();
    CompiledCorpus { program, walk }
}
