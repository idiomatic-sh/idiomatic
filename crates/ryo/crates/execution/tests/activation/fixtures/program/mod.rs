use ryo_execution::inbound::use_cases::execution::{
    interface::ExecutionUseCase, types::*, use_case::ExecutionUseCaseImpl,
};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, Relation},
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{
        ColumnIndex, Direction, Input, JoinColumn, LiteralText, Operand, PlanComparison,
        QueryLiteral, Side, StepId, StepScope, Test,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::sync::Arc;

fn plan(index: usize) -> Plan {
    let node = ColumnIndex(0);
    let operation = if index % 2 == 1 {
        Operation::Axis {
            from: Input::Relation(Relation::Function),
            edges: Input::Relation(Relation::SyntaxChild),
            domain: Input::Relation(Relation::SyntaxTree),
            direction: Direction::Descendant,
            id: node,
            parent: node,
            child: ColumnIndex(1),
            domain_id: node,
            domain_filter: vec![],
            domain_view: vec![],
            output: vec![JoinColumn {
                side: Side::Left,
                column: node,
            }],
        }
    } else {
        let name = ColumnIndex(
            u32::try_from(
                kinds(Relation::Function)
                    .iter()
                    .position(|kind| *kind == CellKind::Symbol)
                    .unwrap(),
            )
            .unwrap(),
        );
        Operation::Select {
            from: Input::Relation(Relation::Function),
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(name),
                right: Operand::Literal(QueryLiteral::Text(LiteralText(
                    format!("{index:08}").into(),
                ))),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            }],
            kept: vec![node],
        }
    };
    Plan {
        parameters: vec![],
        observation_inputs: vec![],
        observations: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation,
        }],
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(0),
            subject: node,
            key: vec![],
            headings: vec![],
        },
    }
}

pub(in crate::fixtures) fn engine() -> ExecutionUseCaseImpl {
    ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    )
}
pub(in crate::fixtures) fn corpus() -> [CorpusInput; 1] {
    [CorpusInput::Available(CorpusEntry {
        executable: ExecutablePermission::Unknown,
        path: FilePath("activation.fixture".into()),
        size: FileSize(0),
        modified: Nanoseconds(0),
    })]
}
pub(in crate::fixtures) fn prepared(
    engine: &ExecutionUseCaseImpl,
    total: usize,
) -> PreparedExecutionProgram {
    engine
        .prepare(PreparedRequest {
            plans: (0..total).map(plan).collect(),
            bindings: vec![vec![]; total],
            rules: (0..total)
                .map(|index| RuleSpec {
                    id: RuleId(format!("activation-{index}").into()),
                    query: QueryBundle {
                        root: QueryDocumentSource {
                            id: QuerySourceId(format!("activation-{index}").into()),
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
                })
                .collect(),
            digests: (0..total).map(|index| Digest([index as u8; 16])).collect(),
            hosted: vec![],
            observations: ObservationIdentity::Independent,
        })
        .unwrap()
}
