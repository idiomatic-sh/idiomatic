//! The workspace entry: families acquired through `InputAdmissions` of an invocation workspace.
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::route::exit::{Exit, released};
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::request::{observed, request};
use crate::application::file_dag::derived::owner::acquisition::family::fixtures::{SMALL_SOURCE, SmallTree, numbers, small_tree};
use crate::application::file_dag::{
    derived::owner::{ViewContext, acquisition::admission::service::AcquiredView},
    relations::RelationOwner,
    fixtures::program_derived_port,
    workspace::{InputAdmissions, InvocationWorkspace, ViewInputs},
};
use crate::domain::{
    derived_views::types::contract::value::PreparedViewUse,
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    table_cells::types::{TreeForm, TreeRows},
    table_model::types::Rows,
    table_texts::types::Texts,
};
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, FileId, NodeId, Relation, Value};
use ryo_values::domain::files::types::SourceView;
use std::{rc::Rc, sync::Arc};

pub(super) fn workspace(kind: DerivedKind, exit: Exit) -> Result<DerivedKind, OperatingError> {
    let owner = Rc::new(RelationOwner::new(crate::application::file_dag::relations::types::ReuseEligibility::Eligible, program_derived_port()));
    let result = through_workspace(kind, exit, &owner);
    owner.release_population();
    released(kind, exit, owner.derived().publication_ownership_bytes());
    result
}
fn through_workspace(
    kind: DerivedKind,
    exit: Exit,
    owner: &Rc<RelationOwner>,
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
        BorrowedRows::Tree(tree)
    } else {
        second_held = owner.resolve(second.as_ref(), &|_| Err(OperatingError::NotRead))?;
        BorrowedRows::from(&second_held)
    };
    let rows = [BorrowedRows::from(&first_held), second_rows];
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
    let plan = Arc::new(owner.derived().prepare_view(&request(kind), &shapes[..len])?);
    let use_ = PreparedViewUse {
        plan,
        inputs: [
            ryo_values::domain::plan_parts::types::InputNumber(0),
            ryo_values::domain::plan_parts::types::InputNumber(1),
        ],
        relations: [None, None],
        len: crate::domain::derived_views::types::InputsUsed(
            u8::try_from(len).map_err(|_| OperatingError::ShapeMismatch)?,
        ),
    };
    let mut published = vec![first.clone()];
    if pair {
        published.push(second.clone());
    }
    let mut workspace = InvocationWorkspace::new(Rc::clone(owner));
    workspace.capture_admitted(published);
    let admissions = workspace.inputs(&rows[..len]);
    match exit {
        Exit::Cancelled => {
            drop(admissions);
            workspace.release_inputs();
            Ok(kind)
        }
        Exit::Succeeds => {
            let view = demand(kind, &admissions, &use_, &mut symbols, false)?;
            assert_eq!(observed(&view)?, kind, "{kind:?}: the leased view reports its family");
            Ok(kind)
        }
        Exit::Errors => {
            if !native {
                let wrong = demand(kind, &admissions, &use_, &mut symbols, true);
                assert!(
                    wrong.is_err(),
                    "{kind:?}: inputs of another family are refused by the owner"
                );
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
        _ => admissions.acquire_derived(use_, context, ViewInputs::Rows),
    }
}

