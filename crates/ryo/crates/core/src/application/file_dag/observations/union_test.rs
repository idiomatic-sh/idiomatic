//! Multiplicity and invocation count are independent of zero/one/many producer grouping.
use super::union::combine;
use crate::application::file_dag::{
    execution::CommonOperatorEngine, executor::PrimitiveExecutor, feed::Feed,
    relations::RelationOwner,
};
use crate::domain::{
    operating::{
        errors::OperatingError,
        types::{Emit, OperatorContext, Room},
    },
    operator::contract::types::OperatorContract,
    table_model::types::{RowCount, Rows, Table},
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::{
    plan_parts::types::{Input, StepId},
    plans::types::{Operation, OperationKind},
};
use std::sync::atomic::{AtomicUsize, Ordering};
#[derive(Debug, Default)]
struct Counted(AtomicUsize);
impl PrimitiveExecutor for Counted {
    fn stream(
        &self,
        contract: &OperatorContract,
        feed: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        self.0.fetch_add(1, Ordering::Relaxed);
        CommonOperatorEngine::fixture().stream(contract, feed, context, room, emit)
    }
    fn collect(
        &self,
        contract: &OperatorContract,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        CommonOperatorEngine::fixture().collect(contract, inputs, context, out)
    }
}
#[test]
fn zero_singleton_and_many_producers_share_one_distinct_union_invocation() {
    let cases: &[(&[&[i64]], &[i64])] = &[
        (&[], &[]),
        (&[&[]], &[]),
        (&[&[1, 1, 2]], &[1, 2]),
        (&[&[1, 1, 2], &[]], &[1, 2]),
        (&[&[], &[1, 1, 2]], &[1, 2]),
        (&[&[1, 2], &[2, 3], &[1, 3]], &[1, 2, 3]),
    ];
    for (given, expected) in cases {
        let owner = RelationOwner::new(true);
        let sources = given
            .iter()
            .map(|rows| {
                owner
                    .computed(Table {
                        kinds: vec![CellKind::Number],
                        cells: rows
                            .iter()
                            .map(|number| Value::Number(Integer(*number)))
                            .collect(),
                        count: RowCount(rows.len()),
                    })
                    .expect("valid publication")
            })
            .collect::<Vec<_>>();
        let schema = crate::domain::operation_schema::types::InputSchema {
            kinds: &[CellKind::Number],
            tree: None,
        };
        let contract = crate::domain::operator::contract::service::prepare_for(
            OperationKind::Union,
            &Operation::Union {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
            },
            &[schema, schema],
            &[],
        )
        .expect("typed union");
        let mut symbols = Symbols::default();
        let mut scratch = OperatorScratch::default();
        let mut memo = Vec::new();
        let mut context = OperatorContext {
            semantic: None,
            prepared_matchers: None,
            prepared_kinds: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        let counted = Counted::default();
        let read = |_| Err(OperatingError::NotRead);
        let published = combine(
            &owner,
            &sources,
            &[CellKind::Number],
            &contract,
            &read,
            &counted,
            &mut context,
        )
        .expect("one union pass");
        let Rows::Computed(table) = owner
            .resolve(published.as_ref(), &read)
            .expect("published output")
        else {
            panic!("union output is computed");
        };
        assert_eq!(
            table.cells,
            expected
                .iter()
                .map(|number| Value::Number(Integer(*number)))
                .collect::<Vec<_>>(),
            "{given:?}"
        );
        assert_eq!(table.count.0, expected.len());
        assert_eq!(
            counted.0.load(Ordering::Relaxed),
            1,
            "one engine-owned distinct state for {given:?}"
        );
    }
}
