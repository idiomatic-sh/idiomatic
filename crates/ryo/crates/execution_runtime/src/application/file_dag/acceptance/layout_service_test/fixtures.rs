//! Authored requests for the schema, publication and refusal controls.
use ryo_execution_domain::domain::observation_schedule::types::RuleSchedule;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::Compiled;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::query_tokens::types::QueryDocumentSource;
use ryo_values::domain::query_tokens::types::QuerySourceId;
use ryo_values::domain::rule_requests::types::RuleParameters;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleApplicability;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;

// Fixtures assemble authored requests only. The production compiler derives every schema,
// retention boundary, reader count and frontier reference asserted below.
pub(super) fn plan(steps: Vec<PlanStep>, frontier: Vec<Input>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        emit: Emit {
            step: StepId(steps.len().saturating_sub(1) as u32),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
        frontier,
        comparison_origins: vec![],
    }
}
pub(super) fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}
pub(super) fn select(from: Input) -> Operation {
    Operation::Select {
        from,
        comparisons: vec![],
        kept: vec![],
    }
}
pub(super) fn native_plan(cross: bool) -> Plan {
    let mut tail = step(if cross {
        Operation::GroupCount {
            from: Input::Step(StepId(0)),
            keys: vec![ColumnIndex(1)],
            distinct: None,
        }
    } else {
        Operation::Project {
            from: Input::Step(StepId(0)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        }
    });
    if cross {
        tail.scope = StepScope::CrossFile;
    }
    plan(
        vec![step(select(Input::Relation(Relation::SyntaxTree))), tail],
        if cross {
            vec![Input::Step(StepId(0))]
        } else {
            vec![]
        },
    )
}
pub(super) fn prepared(
    plan: Plan,
) -> Result<
    Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    OperatingError,
> {
    let rule = RuleSpec {
        id: RuleId("prepared-layout".into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId("prepared-layout".into()),
                text: "".into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    };
    crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![vec![]],
        },
        &[rule],
        &[Digest([0; 16])],
        &[],
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
        ObservationIdentity::Independent,
    )
}
