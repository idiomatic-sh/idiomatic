//! The workspace entry: families acquired through `InputAdmissions` of an invocation workspace.
use ryo_operators::domain::table_texts::types::Texts;
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::request::{
    observed, request,
};
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::route::exit::Exit;
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::{
    SmallTree,
    departed::{Departed, Refused, Seen, departed, read},
    numbers, small_tree,
};
use crate::application::file_dag::{
    derived::owner::{acquisition::admission::service::AcquiredView, service::ViewContext},
    fixtures::program_derived_port,
    relations::service::RelationOwner,
    workspace::{service::InputAdmissions, service::InvocationWorkspace, types::ViewInputs},
};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::table_cells::fixtures::SMALL_SOURCE;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    derived_views::types::contract::value::PreparedViewUse,
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    table_model::types::Rows,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_relations::domain::relations::types::{CellKind, FileId, NodeId, Relation, Value};
use ryo_values::domain::files::types::SourceView;
use std::{rc::Rc, sync::Arc};

pub(super) fn workspace(kind: DerivedKind, exit: Exit) -> Result<Departed, Refused> {
    let owner = Rc::new(RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        program_derived_port(),
    ));
    let mut along = Vec::new();
    let result = through_workspace(kind, exit, &owner, &mut along);
    owner.release_population();
    departed(
        result,
        along,
        None,
        owner.derived().publication_ownership_bytes(),
    )
}
fn through_workspace(
    kind: DerivedKind,
    exit: Exit,
    owner: &Rc<RelationOwner>,
    along: &mut Vec<Seen>,
) -> Result<DerivedKind, OperatingError> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut table = numbers();
    let native = kind == DerivedKind::NativeLookup;
    if native {
        table.kinds[0] = CellKind::Node;
        table.cells[0] = Value::Node(NodeId(0));
        table.cells[4] = Value::Node(NodeId(1));
    }
    let first = owner.computed(table)?;
    let first_held = owner.resolve(first.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let tree = TreeRows {
        elements: &elements,
        file: FileId(0),
        form: TreeForm::Elements,
    };
    let tree_rows = Rows::Tree(tree);
    let second = if native {
        owner.source(Relation::SyntaxTree, &tree_rows)?
    } else {
        owner.computed(numbers())?
    };
    let second_held;
    let second_rows = if native {
        RepresentationInput::Tree(tree)
    } else {
        second_held = owner.resolve(second.as_ref(), &|_| Err(OperatingError::NotRead))?;
        RepresentationInput::from(&second_held)
    };
    let rows = [RepresentationInput::from(&first_held), second_rows];
    let pair = matches!(
        kind,
        DerivedKind::OrderedKeys | DerivedKind::OrderedDfs | DerivedKind::NativeLookup
    );
    let len = if pair { 2 } else { 1 };
    let shapes = [
        InputShape {
            kinds: rows[0].kinds(),
            tree: None,
        },
        InputShape {
            kinds: rows[1].kinds(),
            tree: native.then_some(TreeForm::Elements),
        },
    ];
    let plan = Arc::new(
        owner
            .derived()
            .prepare_view(&request(kind), &shapes[..len])?,
    );
    let use_ = PreparedViewUse {
        plan,
        inputs: [
            ryo_values::domain::plan_parts::types::InputNumber(0),
            ryo_values::domain::plan_parts::types::InputNumber(1),
        ],
        relations: [None, None],
        len: ryo_execution_domain::domain::derived_views::types::InputsUsed(
            u8::try_from(len).map_err(|_| OperatingError::ShapeMismatch)?,
        ),
    };
    let mut published = vec![first.clone()];
    if pair {
        published.push(second.clone());
    }
    let mut workspace = InvocationWorkspace::new(Rc::clone(owner));
    workspace.capture_admitted(
        published
            .into_iter()
            .map(crate::application::file_dag::relations::handle::service::Publication::of)
            .collect(),
    );
    let admissions = workspace.inputs(&rows[..len]);
    match exit {
        Exit::Cancelled => {
            drop(admissions);
            workspace.release_inputs();
            Ok(kind)
        }
        Exit::Succeeds => {
            let view = demand(kind, &admissions, &use_, &mut symbols, false)?;
            along.push(read(observed(&view)));
            Ok(kind)
        }
        Exit::Errors => {
            if !native {
                // Inputs of another family are refused by the owner.
                let wrong = demand(kind, &admissions, &use_, &mut symbols, true);
                along.push(read(wrong.and_then(|view| observed(&view))));
            }
            owner.release_population();
            demand(kind, &admissions, &use_, &mut symbols, false).map(|_| kind)
        }
    }
}
fn demand(
    kind: DerivedKind,
    admissions: &InputAdmissions<'_, '_>,
    use_: &PreparedViewUse,
    symbols: &mut Symbols,
    mismatched: bool,
) -> Result<AcquiredView, OperatingError> {
    let context = ViewContext::default();
    if mismatched {
        let wrong = if matches!(kind, DerivedKind::OrderedKeys | DerivedKind::OrderedDfs) {
            ViewInputs::Rows
        } else {
            ViewInputs::MembershipValues
        };
        return admissions.acquire_derived(use_, context, wrong);
    }
    match kind {
        DerivedKind::OrderedKeys => {
            let texts = Texts {
                symbols: &*symbols,
                source: SourceView(SMALL_SOURCE),
                paths: &[],
                arenas: &[],
            };
            admissions.acquire_derived(use_, context, ViewInputs::Text(&texts))
        }
        DerivedKind::OrderedDfs => {
            let mut query = (&mut *symbols).into();
            admissions.acquire_derived(
                use_,
                context,
                ViewInputs::Dfs {
                    symbols: &mut query,
                    source: SourceView(SMALL_SOURCE),
                    paths: &[],
                    arenas: &[],
                },
            )
        }
        DerivedKind::NativeLookup => admissions
            .try_native_lookup(use_)?
            .ok_or(OperatingError::WrongOperation),
        DerivedKind::Equality
        | DerivedKind::Adjacency
        | DerivedKind::Membership
        | DerivedKind::PartitionOrder
        | DerivedKind::ClosureAdjacency
        | DerivedKind::CycleAdjacency => {
            admissions.acquire_derived(use_, context, ViewInputs::Rows)
        }
    }
}
