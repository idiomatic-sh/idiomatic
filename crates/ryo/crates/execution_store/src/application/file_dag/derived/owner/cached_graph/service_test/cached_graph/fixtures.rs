//! The support of the `cached_graph` cases.
use crate::application::file_dag::relations::service::RelationOwner;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
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

/// The operator contract of the axis of `axis_plan`, its derived views bound through `owner`'s request views.
fn axis_contract(
    owner: &std::rc::Rc<RelationOwner>,
) -> Result<ryo_execution_domain::domain::operator::contract::types::OperatorContract, OperatingError>
{
    let plan = axis_plan(Relation::Function);
    let operation = &plan.steps[0].operation;
    let kinds = ryo_relations::domain::relations::service::kinds(Relation::Function);
    let shape = ryo_execution_domain::domain::operation_schema::types::InputShape {
        kinds,
        tree: ryo_operators::domain::tree_cells::service::native_form(Relation::Function),
    };
    let mut contract = ryo_execution_domain::domain::operator::service::prepare_operator(
        ryo_values::domain::plans::service::kind_of(operation),
        operation,
        &[shape; 3],
        &[],
        ryo_values::domain::plan_parts::types::StepScope::PerFile,
    )?;
    crate::application::file_dag::derived::request::service::RequestViews::new(
        owner.derived().semantic.clone(),
    )
    .bind(&mut contract)?;
    Ok(contract)
}

/// What `prepare_graph` answers for the axis of `axis_plan` on `owner`.
pub(super) fn graph_of(owner: &std::rc::Rc<RelationOwner>) -> Result<bool, OperatingError> {
    let contract = &axis_contract(owner)?;
    let workspace = crate::application::file_dag::workspace::service::InvocationWorkspace::new(
        std::rc::Rc::clone(owner),
    );
    owner
        .derived()
        .prepare_graph(contract, &workspace, &contract.resources, &[])
        .map(|graph| graph.is_some())
}

/// What the admitted views port answers for the axis of `axis_plan` on `owner` over `rows`: the `from` rows at place 0,
/// the stored edges at place 1 and the domain rows at place 2, each captured as its own publication of `Function`.
pub(super) fn admitted_graph_of(
    owner: &std::rc::Rc<RelationOwner>,
    rows: &Rows<'_>,
) -> Result<bool, OperatingError> {
    use crate::application::file_dag::derived::admitted_views::interface::DerivedAdmittedViewsPort;
    use crate::application::file_dag::relations::handle::service::Publication;
    use crate::application::file_dag::workspace::interface::WorkspacePort;
    let contract = &axis_contract(owner)?;
    let published = [
        owner.source(Relation::Function, rows)?,
        owner.source(Relation::Function, rows)?,
        owner.source(Relation::Function, rows)?,
    ];
    let originals = published
        .iter()
        .map(|publication| {
            owner.resolve(publication.as_ref(), &|relation| {
                if relation == Relation::Function {
                    Ok(rows.clone())
                } else {
                    Err(OperatingError::NotRead)
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut workspace = crate::application::file_dag::workspace::service::InvocationWorkspace::new(
        std::rc::Rc::clone(owner),
    );
    WorkspacePort::capture_admitted(
        &mut workspace,
        published
            .iter()
            .map(|publication| Publication::of(publication.clone()))
            .collect::<Vec<_>>()
            .into(),
    );
    let inputs = originals
        .iter()
        .map(ryo_execution_domain::domain::table_model::types::RepresentationInput::from)
        .collect::<Vec<_>>();
    let admitted = WorkspacePort::inputs(&workspace, &inputs);
    DerivedAdmittedViewsPort::prepare_graph(&admitted, contract, &contract.resources, &originals)
        .map(|graph| {
            matches!(
                graph,
                crate::application::file_dag::relations::types::Sought::Found(_)
            )
        })
}
