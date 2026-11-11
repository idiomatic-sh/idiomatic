//! The support of the `cached_graph` cases.
use crate::application::file_dag::relations::RelationOwner;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};

pub(super) const SOURCE: Input = Input::Relation(Relation::Function);

pub(super) const ENDPOINTS: [ColumnIndex; 2] = [ColumnIndex(0), ColumnIndex(1)];

/// One axis over `relation`, its only step and its emit.
pub(super) fn axis_plan(relation: Relation) -> ryo_values::domain::plans::types::Plan {
    use ryo_values::domain::plan_parts::types::{Direction, StepId, StepScope};
    use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: vec![],
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Axis {
                from: Input::Relation(relation),
                edges: Input::Relation(relation),
                domain: Input::Relation(relation),
                direction: Direction::Descendant,
                id: ColumnIndex(0),
                parent: ColumnIndex(0),
                child: ColumnIndex(0),
                domain_id: ColumnIndex(0),
                domain_filter: vec![],
                domain_view: vec![],
                output: vec![],
            },
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }
}

/// What `prepare_graph` answers for the axis of `axis_plan` on `owner`.
pub(super) fn graph_of(owner: &std::rc::Rc<RelationOwner>) -> Result<bool, OperatingError> {
    // The axis step's operator contract, its derived views bound through the store's request views.
    let plan = axis_plan(Relation::Function);
    let operation = &plan.steps[0].operation;
    let kinds = ryo_relations::domain::relations::service::kinds(Relation::Function);
    let shape = ryo_execution_domain::domain::operation_schema::types::InputShape {
        kinds,
        tree: ryo_execution_domain::domain::table_cells::service::native_form(Relation::Function),
    };
    let mut contract = ryo_execution_domain::domain::operator::service::prepare_operator(
        ryo_values::domain::plans::service::kind_of(operation),
        operation,
        &[shape; 3],
        &[],
        ryo_values::domain::plan_parts::types::StepScope::PerFile,
    )?;
    crate::application::file_dag::derived::request::RequestViews::new(
        owner.derived().semantic.clone(),
    )
    .bind(&mut contract)?;
    let contract = &contract;
    let workspace = crate::application::file_dag::workspace::InvocationWorkspace::new(
        std::rc::Rc::clone(owner),
    );
    owner
        .derived()
        .prepare_graph(contract, &workspace, &contract.resources, &[])
        .map(|graph| graph.is_some())
}
