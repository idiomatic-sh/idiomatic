//! Behavioral proof of `admitted_views`: the views an invocation's admitted inputs demand are acquired from the owner,
//! and a demand the inputs do not fit is refused before anything is built.
use super::fixtures::{context, owner, table, use_};
use crate::application::file_dag::relations::handle::service::Publication;
use crate::application::file_dag::workspace::service::InvocationWorkspace;
use crate::application::file_dag::workspace::types::ViewInputs;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_execution_domain::domain::derived_views::types::ViewRequest;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::rc::Rc;

/// What is demanded of the admitted publication of the one-column table `3, 1, 3`.
#[derive(Clone, Copy, Debug)]
enum Demand {
    /// Its equality index on column 0.
    Rows,
    /// A view that reads none of the inputs.
    NoInput,
    /// A native lookup, through the equality on column 0.
    LookupOfAnEquality,
    /// The partitioned closure of rows the publication did not resolve.
    PartitionsOfOtherRows,
}

type Input = Demand;

/// The rows the acquired index holds.
type Value = usize;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("admitted_rows_acquire_their_index"),
        word: Word::Succeeds,
        input: || Demand::Rows,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_view_reading_no_input_is_the_wrong_operation"),
        word: Word::Refuses,
        input: || Demand::NoInput,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("a_native_lookup_of_another_kind_is_the_wrong_operation"),
        word: Word::Refuses,
        input: || Demand::LookupOfAnEquality,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("partitions_of_rows_never_admitted_are_not_read"),
        word: Word::Refuses,
        input: || Demand::PartitionsOfOtherRows,
        expect: Expect::Error(OperatingError::NotRead),
    },
];

/// Publishes `3, 1, 3`, captures it as the invocation's input and demands `demand` of its admission.
fn subject(demand: Demand) -> Result<Value, Failure> {
    let owner = owner();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    let (mut symbols, mut scratch, mut memo) = (Symbols::default(), Default::default(), vec![]);
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let published = owner.computed(table(&[3, 1, 3], 1))?;
    let rows = owner.resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))?;
    workspace.capture_admitted(vec![Publication::of(published.clone())]);
    let inputs = [RepresentationInput::from(&rows)];
    let admissions = workspace.inputs(&inputs);
    let equality = use_(
        &ViewRequest::Equality {
            columns: vec![ColumnIndex(0)],
        },
        1,
        1,
    );
    let view = match demand {
        Demand::Rows => admissions.acquire_rows(&equality, &mut context)?,
        Demand::NoInput => admissions.acquire_derived(
            &use_(
                &ViewRequest::Equality {
                    columns: vec![ColumnIndex(0)],
                },
                1,
                0,
            ),
            crate::application::file_dag::derived::owner::service::ViewContext::default(),
            ViewInputs::Rows,
        )?,
        Demand::LookupOfAnEquality => admissions
            .try_native_lookup(&equality)?
            .ok_or(OperatingError::NotRun)?,
        Demand::PartitionsOfOtherRows => {
            let other = table(&[3, 1, 3], 1);
            let order = use_(
                &ViewRequest::PartitionOrder {
                    keys: vec![ColumnIndex(0)],
                },
                1,
                1,
            );
            let closure = use_(
                &ViewRequest::ClosureAdjacency {
                    source: ColumnIndex(0),
                    destination: ColumnIndex(0),
                },
                1,
                1,
            );
            admissions.partitioned(
                &order,
                &closure,
                RepresentationInput::Computed(&other),
                &mut context,
            )?;
            return Err(OperatingError::WrongBinding);
        }
    };
    let DerivedView::Equality(index) = view.view()? else {
        return Err(OperatingError::WrongOperation);
    };
    Ok(index.rows.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
