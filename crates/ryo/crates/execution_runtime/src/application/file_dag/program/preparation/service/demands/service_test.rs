//! The derived needs preparation reads off a compiled program: the walk the store no longer makes, pinned to what the store's
//! own compilation of the same plans answered before it moved here.
use super::service::derived_needs;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::owner::cache::types::GraphAdmission;
use ryo_execution_store::application::file_dag::derived::prepared::RuleDerivedNeeds;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Direction, Input as PlanInput, StepId, StepScope,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

/// The plans of a request, one per rule.
#[derive(Clone, Copy)]
enum Request {
    /// A select over the functions, one over the comments, and an axis over the functions.
    SelectsAndAnAxis,
}
type Input = Request;
/// Each rule's one relation read and whether it walks a graph, in rule order.
type Value = [(Relation, GraphAdmission); 3];
type Failure = OperatingError;

/// A plan of one per-file step and its emit.
fn plan(operation: Operation) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        frontier: vec![],
        comparison_origins: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}
fn select(relation: Relation) -> Plan {
    plan(Operation::Select {
        from: PlanInput::Relation(relation),
        comparisons: vec![],
        kept: vec![],
    })
}
fn axis(relation: Relation) -> Plan {
    plan(Operation::Axis {
        from: PlanInput::Relation(relation),
        edges: PlanInput::Relation(relation),
        domain: PlanInput::Relation(relation),
        direction: Direction::Descendant,
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(0),
        domain_id: ColumnIndex(0),
        domain_filter: vec![],
        domain_view: vec![],
        output: vec![],
    })
}
/// A rule's needs as one relation and its walk, refused when it reads other than one relation.
fn one(needs: &RuleDerivedNeeds) -> Result<(Relation, GraphAdmission), Failure> {
    match &*needs.relations {
        [relation] => Ok((*relation, needs.graph)),
        _ => Err(OperatingError::ShapeMismatch),
    }
}

/// The needs preparation reads off the program the plans compile to.
fn subject(request: Input) -> Result<Value, Failure> {
    use ryo_execution_domain::domain::source_partitions::types::source_partitions::value::SourcePartitions;
    use ryo_values::domain::{
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        runs::types::{RuleApplicability, RuleId},
    };
    let plans = match request {
        Request::SelectsAndAnAxis => vec![
            select(Relation::Function),
            select(Relation::Comment),
            axis(Relation::Function),
        ],
    };
    let rules = (0..plans.len())
        .map(|rule| RuleSpec {
            id: RuleId(format!("rule-{rule}").into()),
            query: QueryBundle {
                root: QueryDocumentSource {
                    id: QuerySourceId(format!("rule-{rule}").into()),
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
        .collect::<Vec<_>>();
    let bindings = vec![vec![]; plans.len()];
    let operators = crate::application::file_dag::service::prepare_with_schemas(
        &plans,
        &bindings,
        &[],
        &ryo_relations::domain::relations::service::kinds,
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
    )?;
    let borrowed = plans.iter().map(Ok).collect::<Vec<_>>();
    let partitions = SourcePartitions::new(
        &rules,
        &borrowed,
        &bindings,
        &[],
        &vec![vec![]; plans.len()],
    )?;
    let graph =
        ryo_execution_domain::domain::file_program::service::compile_prepared(&plans, &operators);
    let program = crate::application::file_dag::program::preparation::service::compile_program(
        &plans,
        graph,
        &partitions,
        &operators,
    )?;
    let needs = derived_needs(&program)?
        .iter()
        .map(one)
        .collect::<Result<Vec<_>, _>>()?;
    needs.try_into().map_err(|_| OperatingError::ShapeMismatch)
}

/// The rows: what the store's own compilation answered for these plans before the walk moved to preparation.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("each_rule_reads_its_relation_and_only_the_axis_walks_a_graph"),
    word: Word::Succeeds,
    input: || Request::SelectsAndAnAxis,
    expect: Expect::Value([
        (Relation::Function, GraphAdmission::Disabled),
        (Relation::Comment, GraphAdmission::Disabled),
        (Relation::Function, GraphAdmission::Enabled),
    ]),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
