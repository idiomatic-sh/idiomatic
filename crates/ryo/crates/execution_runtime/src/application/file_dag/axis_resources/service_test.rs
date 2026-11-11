//! Real borrowed-index traversal, duplicate order and admission timing controls.
use super::service::*;
use super::view::service::AxisResource;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::types::Table;
use ryo_execution_domain::domain::derived_views::types::{
    DerivedInputs, DerivedScratch, MembershipSpace,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::resources::types::AxisResources;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;
use std::sync::Arc;
use {
    ryo_execution_domain::domain::operator::contract::views::types::PreparedAxis,
    ryo_execution_domain::domain::table_reach::borrowed::service::follow,
    ryo_execution_domain::domain::table_reach::borrowed::service::reach_breadth,
    ryo_execution_domain::domain::table_reach::types::Walk,
    ryo_values::domain::rows::service::push_row,
};

fn number(value: Integer) -> Cell {
    Cell::Number(value)
}

fn table(width: ColumnCount, rows: &[&[Integer]]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; width.0],
        ..Table::default()
    };
    for row in rows {
        push_row(
            &mut table,
            &row.iter().copied().map(number).collect::<Vec<_>>(),
        );
    }
    table
}

fn program(direction: Direction) -> AxisResources {
    AxisResources {
        id: ColumnIndex(0),
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        has_filter:
            ryo_execution_domain::domain::operator::contract::resources::types::Filtering::Unfiltered,
        domain_view: Vec::new(),
        output: Vec::new(),
        direction,
    }
}

fn acquired(
    kind: ryo_derived::domain::request::types::DerivedKind,
    request: ryo_execution_domain::domain::derived_views::types::ViewRequest,
    rows: RepresentationInput<'_>,
) -> AcquiredView {
    let plan = Arc::new(
        ryo_execution_store::application::file_dag::derived::prepare::service::view(
            Arc::new(ryo_execution_store::application::file_dag::fixtures::derived()),
            &request,
            &[
                ryo_execution_domain::domain::operation_schema::types::InputShape {
                    kinds: rows.kinds(),
                    tree: None,
                },
            ],
        )
        .unwrap(),
    );
    let owner = ryo_execution_store::application::file_dag::relations::service::RelationOwner::fixture(
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Ineligible,
        Arc::new(ryo_execution_store::application::file_dag::fixtures::derived()),
    );
    let RepresentationInput::Computed(table) = rows else {
        panic!("computed fixture")
    };
    let publication = owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Computed(table.clone())).unwrap();
    let held = owner
        .rows(&publication, &|_| Err(OperatingError::NotRead))
        .unwrap();
    let rows = RepresentationInput::from(&held);
    let proof = owner
        .admit(&publication, rows)
        .and_then(|admitted| admitted)
        .unwrap();
    let inputs = [rows];
    let data = if kind == ryo_derived::domain::request::types::DerivedKind::Membership {
        DerivedInputs::Membership {
            rows,
            space: MembershipSpace::Values,
        }
    } else {
        DerivedInputs::Rows(&inputs)
    };
    owner
        .derived()
        .acquire_stored(
            &plan,
            &owner
                .derived()
                .admit_view(&plan, &owner, &[&proof])
                .unwrap(),
            ryo_execution_store::application::file_dag::derived::owner::service::ViewContext::default(),
            data,
            &mut DerivedScratch::default(),
            None,
        )
        .unwrap()
}

/// An axis program over computed edge and domain rows, walked from `start`.
#[derive(Clone, Copy, Debug)]
struct Axis {
    /// The program's direction.
    direction: Direction,
    /// The (parent, child) edge rows.
    edges: &'static [[i64; 2]],
    /// The domain ids.
    domain: &'static [i64],
    /// Whether the equality and membership views are acquired and handed over.
    handles: bool,
    /// The id the direct follow and the breadth walk start from.
    start: i64,
}

/// What the prepared axis answers.
#[derive(Debug, PartialEq)]
enum Prepared {
    /// The empty-domain resource, prepared without derived handles.
    Empty,
    /// The borrowed indexed resource: the direct children of `start` and its breadth reach.
    Indexed {
        direct: Cow<'static, [i64]>,
        reached: Cow<'static, [i64]>,
    },
    /// Any other resource, named.
    Other(&'static str),
}

type Input = Axis;
type Value = Prepared;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("borrowed_axis_keeps_duplicate_direct_edges_and_breadth_order"),
        word: Word::Succeeds,
        input: || Axis {
            direction: Direction::Child,
            edges: &[[1, 2], [1, 2], [2, 3]],
            domain: &[2, 3],
            handles: true,
            start: 1,
        },
        expect: Expect::Value(Prepared::Indexed {
            direct: Cow::Borrowed(&[2, 2]),
            reached: Cow::Borrowed(&[2, 3]),
        }),
    },
    Case {
        name: CaseName("unsupported_and_empty_gates_precede_derived_handle_requirement"),
        word: Word::Succeeds,
        input: || Axis {
            direction: Direction::Child,
            edges: &[[1, 2]],
            domain: &[],
            handles: false,
            start: 1,
        },
        expect: Expect::Value(Prepared::Empty),
    },
    Case {
        name: CaseName("a_deep_axis_is_refused_before_the_derived_handles_are_required"),
        word: Word::Refuses,
        input: || Axis {
            direction: Direction::Deep,
            edges: &[[1, 2]],
            domain: &[2],
            handles: false,
            start: 1,
        },
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

fn integers(cells: &[Cell]) -> Cow<'static, [i64]> {
    cells
        .iter()
        .map(|cell| match cell {
            Cell::Number(Integer(value)) => *value,
            other => panic!("a number cell, not {other:?}"),
        })
        .collect()
}

/// Prepares the axis and, when indexed, follows and walks it from `start`.
fn subject(axis: Input) -> Result<Value, Failure> {
    let edge_rows: Vec<[Integer; 2]> = axis
        .edges
        .iter()
        .map(|[parent, child]| [Integer(*parent), Integer(*child)])
        .collect();
    let edge_refs: Vec<&[Integer]> = edge_rows.iter().map(|row| row.as_slice()).collect();
    let domain_rows: Vec<[Integer; 1]> = axis.domain.iter().map(|id| [Integer(*id)]).collect();
    let domain_refs: Vec<&[Integer]> = domain_rows.iter().map(|row| row.as_slice()).collect();
    let edges = table(ColumnCount(2), &edge_refs);
    let domain = table(ColumnCount(1), &domain_refs);
    let (equality, membership) = if axis.handles {
        (
            Some(acquired(
                ryo_derived::domain::request::types::DerivedKind::Equality,
                ryo_execution_domain::domain::derived_views::types::ViewRequest::Equality {
                    columns: vec![ColumnIndex(0)],
                },
                RepresentationInput::Computed(&edges),
            )),
            Some(acquired(
                ryo_derived::domain::request::types::DerivedKind::Membership,
                ryo_execution_domain::domain::derived_views::types::ViewRequest::Membership {
                    column: ColumnIndex(0),
                },
                RepresentationInput::Computed(&domain),
            )),
        )
    } else {
        (None, None)
    };
    let resource = prepare_axis(
        &program(axis.direction),
        RepresentationInput::Computed(&edges),
        RepresentationInput::Computed(&domain),
        equality,
        membership,
    )?;
    match &resource {
        AxisResource::Empty => Ok(Prepared::Empty),
        AxisResource::Native { .. } => Ok(Prepared::Other("native")),
        AxisResource::Indexed { .. } => {
            #[cfg(feature = "diagnostic-row-progress")]
            let viewed =
                resource.view(&ryo_execution_domain::domain::operating::types::IgnoredProgress)?;
            #[cfg(not(feature = "diagnostic-row-progress"))]
            let viewed = resource.view()?;
            let PreparedAxis::Indexed {
                edges: view, to, ..
            } = viewed
            else {
                return Ok(Prepared::Other("an indexed resource without an indexed view"));
            };
            let mut key = Vec::new();
            let mut direct = Vec::new();
            follow(&view, to, number(Integer(axis.start)), &mut key, &mut direct);
            let mut walk = Walk::default();
            reach_breadth(&view, to, number(Integer(axis.start)), &mut walk);
            Ok(Prepared::Indexed {
                direct: integers(&direct),
                reached: integers(&walk.reached),
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
