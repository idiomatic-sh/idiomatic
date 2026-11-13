use super::service::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::derived_views::service::validate;
use ryo_execution_domain::domain::operator::contract::resources::types::Filtering;
use ryo_execution_domain::domain::operator::contract::resources::{
    types::AxisResources, types::JoinResources,
};
use ryo_execution_domain::domain::{
    derived_views::types::{contract::value::DerivedSlots, *},
    operating::errors::OperatingError,
    operator::contract::resources::{types::IndexColumns, types::ResourceProgram},
};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, InputNumber};
use std::borrow::Cow;
use std::sync::Arc;

fn kinds() -> Vec<Vec<CellKind>> {
    vec![
        vec![CellKind::Number, CellKind::Number],
        vec![CellKind::Number, CellKind::Number],
        vec![CellKind::Number],
    ]
}

/// A resource program the views bind.
#[derive(Clone, Copy, Debug)]
enum Program {
    /// The index of `input` over every column.
    WholeIndex { input: usize },
    /// The index of `input` over one column.
    OneColumnIndex { input: usize, column: u32 },
    /// An unfiltered join of column 0 to column 1.
    Join,
    /// An unfiltered parent axis from column 0 to column 1 over domain column 0.
    ParentAxis,
}

/// The programs bound in turn on one set of views, each over the first `width` input schemas.
#[derive(Clone, Copy, Debug)]
struct Binds(&'static [(Program, usize)]);

/// One bound derived slot.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Slot {
    kind: DerivedKind,
    /// The inputs the primary use is built over, and how many of them it uses.
    inputs: [usize; 2],
    used: u8,
    /// The columns of an equality request.
    equality: Option<Cow<'static, [u32]>>,
    /// The refusal the prepared metadata holds, and the one validation answers.
    refusal: Option<OperatingError>,
    validated: Option<OperatingError>,
    /// Whether binding the program again reaches the same plan, and the same primary use.
    same_plan: bool,
    same_use: bool,
}

/// What one program bound: its slots, and the views' plan count after it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Bind {
    slots: Cow<'static, [Slot]>,
    plans: usize,
}

/// Every bind, and the size of a family's slots.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Bound {
    binds: Cow<'static, [Bind]>,
    slot_bytes: usize,
}

type Input = Binds;
type Value = Bound;
type Failure = OperatingError;

/// A bound slot whose use reads `inputs`, `used` of them, again shared on a second bind.
const fn shared(kind: DerivedKind, inputs: [usize; 2], used: u8) -> Slot {
    Slot {
        kind,
        inputs,
        used,
        equality: None,
        refusal: None,
        validated: None,
        same_plan: true,
        same_use: true,
    }
}

/// A family's slots are one handle per derived kind: 9 kinds of 8 bytes.
const SLOT_BYTES: usize = 72;

/// The shared equality slot over inputs 1 and 0 that looks up `columns`.
const fn equality(columns: &'static [u32]) -> Slot {
    Slot {
        kind: DerivedKind::Equality,
        inputs: [1, 0],
        used: 1,
        equality: Some(Cow::Borrowed(columns)),
        refusal: None,
        validated: None,
        same_plan: true,
        same_use: true,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("missing_input_is_prepared_but_refused_only_when_the_view_is_demanded"),
        word: Word::Succeeds,
        input: || Binds(&[(Program::WholeIndex { input: 1 }, 1)]),
        expect: Expect::Value(Bound {
            binds: Cow::Borrowed(&[Bind {
                slots: Cow::Borrowed(&[Slot {
                    kind: DerivedKind::Equality,
                    inputs: [1, 0],
                    used: 1,
                    equality: Some(Cow::Borrowed(&[])),
                    refusal: Some(OperatingError::MissingInput { position: 2 }),
                    validated: Some(OperatingError::MissingInput { position: 2 }),
                    same_plan: true,
                    same_use: true,
                }]),
                plans: 1,
            }]),
            slot_bytes: SLOT_BYTES,
        }),
    },
    Case {
        name: CaseName("plain_join_and_axis_bind_finite_ordered_inputs_and_share_identical_plans"),
        word: Word::Succeeds,
        input: || Binds(&[(Program::Join, 2), (Program::ParentAxis, 3)]),
        expect: Expect::Value(Bound {
            binds: Cow::Borrowed(&[
                Bind {
                    slots: Cow::Borrowed(&[
                        equality(&[1]),
                        shared(DerivedKind::NativeLookup, [0, 1], 2),
                    ]),
                    plans: 3,
                },
                Bind {
                    slots: Cow::Borrowed(&[
                        equality(&[1]),
                        shared(DerivedKind::Adjacency, [1, 0], 1),
                        shared(DerivedKind::Membership, [2, 1], 2),
                    ]),
                    plans: 5,
                },
            ]),
            slot_bytes: SLOT_BYTES,
        }),
    },
    Case {
        name: CaseName("family_slots_hold_compact_shared_binding_handles"),
        word: Word::Succeeds,
        input: || {
            Binds(&[(
                Program::OneColumnIndex {
                    input: 1,
                    column: 0,
                },
                3,
            )])
        },
        expect: Expect::Value(Bound {
            binds: Cow::Borrowed(&[Bind {
                slots: Cow::Borrowed(&[equality(&[0])]),
                plans: 1,
            }]),
            slot_bytes: SLOT_BYTES,
        }),
    },
];

/// The resource program a test program names.
fn program(program: Program) -> ResourceProgram {
    match program {
        Program::WholeIndex { input } => ResourceProgram::Index {
            input: InputNumber(input),
            columns: IndexColumns::Whole,
        },
        Program::OneColumnIndex { input, column } => ResourceProgram::Index {
            input: InputNumber(input),
            columns: IndexColumns::Columns(vec![ColumnIndex(column)]),
        },
        Program::Join => ResourceProgram::Join(JoinResources {
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(1)],
            view: vec![ColumnIndex(0)],
            has_filter: Filtering::Unfiltered,
            reach: None,
        }),
        Program::ParentAxis => ResourceProgram::Axis(AxisResources {
            id: ColumnIndex(0),
            parent: ColumnIndex(0),
            child: ColumnIndex(1),
            domain_id: ColumnIndex(0),
            has_filter: Filtering::Unfiltered,
            domain_view: vec![],
            output: vec![],
            direction: Direction::Parent,
        }),
    }
}

/// Binds each program twice on one set of views and reads every bound slot.
fn subject(binds: Input) -> Result<Value, Failure> {
    let mut views = RequestViews::new(Arc::new(crate::application::file_dag::fixtures::derived()));
    let schema = kinds();
    let mut bound = Vec::new();
    for &(named, width) in binds.0 {
        let resources = program(named);
        let first = views.bind_resources(&resources, &schema[..width])?;
        let second = views.bind_resources(&resources, &schema[..width])?;
        let slots = DerivedKind::ALL
            .iter()
            .filter_map(|&kind| {
                let group = first[kind.ordinal()].as_ref()?;
                let again = second[kind.ordinal()].as_ref()?;
                let primary = &group.primary;
                let equality = if let ViewRequest::Equality { columns } = &primary.plan.request {
                    Some(Cow::Owned(columns.iter().map(|column| column.0).collect()))
                } else {
                    None
                };
                Some(Slot {
                    kind,
                    inputs: primary.inputs.map(|input| input.0),
                    used: primary.len.0,
                    equality,
                    refusal: primary.plan.metadata.as_ref().err().cloned(),
                    validated: validate(kind, &primary.plan).err(),
                    same_plan: Arc::ptr_eq(&primary.plan, &again.primary.plan),
                    same_use: Arc::ptr_eq(primary, &again.primary),
                })
            })
            .collect();
        bound.push(Bind {
            slots: Cow::Owned(slots),
            plans: views.count().0,
        });
    }
    Ok(Bound {
        binds: Cow::Owned(bound),
        slot_bytes: size_of::<DerivedSlots>(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
