//! Multiplicity and invocation count are independent of zero/one/many producer grouping.
use super::union::combine;
use crate::application::file_dag::{
    execution::CommonOperatorEngine,
    executor::{PrimitiveExecutor, ScopedExecutor},
    feed::Feed,
    relations::RelationOwner,
};
use crate::domain::{
    operating::{
        errors::OperatingError,
        types::{Emit, OperatorContext, Room},
    },
    operator::contract::types::OperatorContract,
    table_model::types::Rows,
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::{
    plan_parts::types::{Input, StepId},
    plans::types::{Operation, OperationKind},
    rows::types::RowCount,
};
use std::sync::atomic::{AtomicUsize, Ordering};
#[derive(Debug)]
struct Counted {
    calls: AtomicUsize,
    engine: CommonOperatorEngine<'static>,
}
impl Default for Counted {
    fn default() -> Self {
        Self {
            calls: AtomicUsize::default(),
            engine: CommonOperatorEngine::fixture(),
        }
    }
}
impl PrimitiveExecutor for Counted {
    fn execute(
        &self,
        call: crate::application::file_dag::executor::KernelCall<'_, '_, '_>,
    ) -> Result<
        crate::application::file_dag::executor::KernelAnswer,
        crate::application::file_dag::executor::KernelFault,
    > {
        if matches!(
            &call,
            crate::application::file_dag::executor::KernelCall::Registered { .. }
        ) {
            self.calls.fetch_add(1, Ordering::Relaxed);
        }
        self.engine.execute(call)
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
        let counted = Counted::default();
        let owner = std::rc::Rc::new(RelationOwner::new(true, counted.derived_port()));
        let executor = ScopedExecutor::new(&counted, std::rc::Rc::clone(&owner))
            .expect("the scoped owner uses the same derived port");
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
        let schema = crate::domain::operation_schema::types::InputShape {
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
        let read = |_| Err(OperatingError::NotRead);
        let published = combine(
            &sources,
            &[CellKind::Number],
            &contract,
            &read,
            &executor,
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
            counted.calls.load(Ordering::Relaxed),
            1,
            "one engine-owned distinct state for {given:?}"
        );
    }
}
