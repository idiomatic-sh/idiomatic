//! The plan shapes and the compiled corpus every coverage proof reads.
use super::super::plan;
use super::heads::{FUNCTION, ROW_KEY, TEXT_A, equal, from, operations};
use super::sources::Sources;
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation,
};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, LiteralText, ParameterName, RepeatedRows, StepId,
};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::{BindingCell, BindingTable, ParameterBinding};
use ryo_values::domain::query_bindings::types::{ParameterType, PlanParameter};
use ryo_values::domain::query_operators::types::OperationKind;
pub(crate) const SETS_PARAMETER: &str = "sets";
pub(crate) const ROWS_PARAMETER: &str = "rows";
pub(crate) const ROW_VALUE: &str = "value";
pub(crate) const HOSTED_RELATION: &str = "hosted";
pub(crate) const HOSTED_COLUMN: &str = "n";
/// A plan and the bindings of its rule.
pub(crate) struct Planned {
    pub(crate) plan: Plan,
    pub(crate) bindings: Vec<ParameterBinding>,
}
pub(crate) fn operation_plan(operation: Operation) -> Planned {
    let kind = kind_of(&operation);
    let mut plan = plan(operation, StepScope::PerFile);
    let mut bindings = vec![];
    if kind == OperationKind::Rank {
        plan.parameters.push(PlanParameter {
            name: ParameterName(SETS_PARAMETER.into()),
            kind: ParameterType::TextSets,
        });
        bindings.push(ParameterBinding::TextSets(vec![vec![LiteralText(
            TEXT_A.into(),
        )]]));
    }
    if kind == OperationKind::Rows {
        plan.parameters.push(PlanParameter {
            name: ParameterName(ROWS_PARAMETER.into()),
            kind: ParameterType::Tables,
        });
        bindings.push(ParameterBinding::Tables(vec![BindingTable(vec![
            BindingCell {
                key: LiteralText(ROW_KEY.into()),
                value: LiteralText(ROW_VALUE.into()),
            },
        ])]));
    }
    Planned { plan, bindings }
}
/// Plans beside the variants: relation heads that share, one that does not, a Step input,
/// a fused continuation and a project-stage head.
pub(crate) fn shapes() -> Vec<Plan> {
    let f = from(FUNCTION);
    let select = |comparisons| Operation::Select {
        from: f,
        comparisons,
        kept: vec![],
    };
    let mut plans = vec![
        plan(select(vec![]), StepScope::PerFile),
        plan(select(vec![equal(ColumnIndex(0))]), StepScope::PerFile),
        // A column past the relation's width is no shared head: it keeps its own representation.
        plan(
            Operation::Project {
                from: f,
                columns: vec![ColumnIndex(
                    u32::try_from(
                        ryo_relations::domain::relations::service::columns(FUNCTION).len(),
                    )
                    .unwrap(),
                )],
                distinct: RepeatedRows::Kept,
            },
            StepScope::PerFile,
        ),
        plan(
            Operation::Project {
                from: f,
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            },
            StepScope::PerFile,
        ),
    ];
    let mut dependent = plan(select(vec![]), StepScope::PerFile);
    dependent
        .steps
        .push(ryo_values::domain::plans::types::PlanStep {
            operation: Operation::Select {
                from: Input::Step(StepId(0)),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
            scope: StepScope::PerFile,
        });
    dependent.emit.step = StepId(1);
    plans.push(dependent);
    // Its head is its own graph node (a different predicate from every other plan), so the
    // projection after it is its only consumer and fuses into the same region.
    let mut continuation = plan(select(vec![equal(ColumnIndex(1))]), StepScope::PerFile);
    continuation
        .steps
        .push(ryo_values::domain::plans::types::PlanStep {
            operation: Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Kept,
            },
            scope: StepScope::PerFile,
        });
    continuation.emit.step = StepId(1);
    plans.push(continuation);
    let mut project = plan(select(vec![]), StepScope::CrossFile);
    project.frontier.push(f);
    plans.push(project);
    plans
}
pub(crate) struct Corpus {
    pub program: ExecutableProgram,
    pub walk: ryo_execution_domain::domain::shared_walk::types::prepared_walk::value::PreparedWalk,
}
/// Every variant and shape through the normal operator, graph, program and walk compilers.
pub(crate) fn compiled_operations() -> Corpus {
    let mut plans = Vec::new();
    let mut bindings = Vec::new();
    for operation in operations() {
        let planned = operation_plan(operation);
        plans.push(planned.plan);
        bindings.push(planned.bindings);
    }
    for shape in shapes() {
        plans.push(shape);
        bindings.push(vec![]);
    }
    let hosted = vec![HostRelation {
        name: HostName(HOSTED_RELATION.into()),
        columns: vec![HostColumn {
            name: HostColumnName(HOSTED_COLUMN.into()),
            kind: HostKind::Number,
        }],
        rows: vec![],
    }];
    let operators = crate::application::file_dag::service::prepare_with_schemas(
        &plans,
        &bindings,
        &hosted,
        &ryo_relations::domain::relations::service::kinds,
        crate::application::file_dag::fixtures::real_derived_port(),
    )
    .unwrap();
    let graph =
        ryo_execution_domain::domain::file_program::service::compile_prepared(&plans, &operators);
    let program = ryo_execution_domain::domain::executable_program::service::compile(
        &plans,
        graph,
        &Sources,
        &operators,
        ryo_execution_domain::domain::matching::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    )
    .unwrap();
    let mut walk = ryo_execution_domain::domain::shared_walk::types::prepared_walk::value::PreparedWalk::compile(&plans, &bindings);
    walk.retain_program_consumers(&program).unwrap();
    Corpus { program, walk }
}
/// Authored plans through the normal operator, graph and program compilers, without a walk.
pub(crate) fn compile_program(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
) -> ExecutableProgram {
    let hosted = Vec::new();
    let operators = crate::application::file_dag::service::prepare_with_schemas(
        plans,
        bindings,
        &hosted,
        &ryo_relations::domain::relations::service::kinds,
        crate::application::file_dag::fixtures::real_derived_port(),
    )
    .unwrap();
    let graph =
        ryo_execution_domain::domain::file_program::service::compile_prepared(plans, &operators);
    ryo_execution_domain::domain::executable_program::service::compile(
        plans,
        graph,
        &Sources,
        &operators,
        ryo_execution_domain::domain::matching::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    )
    .unwrap()
}
