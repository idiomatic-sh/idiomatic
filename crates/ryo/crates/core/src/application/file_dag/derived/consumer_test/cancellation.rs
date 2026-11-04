//! Global required views release on both untouched and refusing final consumers.
use super::*;
use crate::application::file_dag::{executor::PrimitiveExecutor, feed::Feed};
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, Room},
};
use crate::domain::operator::contract::types::OperatorContract;
use std::cell::Cell;

#[derive(Debug)]
struct Refuse(std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>);
impl PrimitiveExecutor for Refuse {
    fn native_classes(
        &self,
        _: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        _: &crate::application::file_dag::executor::NativeClassBatch<'_>,
        _: &mut crate::domain::shared_walk::types::MergedWalk,
        _: crate::domain::shared_walk::types::Walking<'_>,
    ) -> Result<(), OperatingError> {
        Err(OperatingError::NotRead)
    }
    fn stream_representation(
        &self,
        _: &OperatorContract,
        _: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        _: &mut crate::application::file_dag::service::GraphInputs<'_, '_>,
        _: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        _: &mut OperatorContext<'_>,
        _: &mut Room,
        _: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        Err(OperatingError::NotRead)
    }
    fn keep_representation(
        &self,
        _: &OperatorContract,
        _: &crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        _: &mut crate::application::file_dag::service::GraphInputs<'_, '_>,
        _: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        _: &mut OperatorContext<'_>,
        _: &mut Room,
    ) -> Result<crate::application::file_dag::relations::PublishedInput, OperatingError> {
        Err(OperatingError::NotRead)
    }
    fn ordered_existence(
        &self,
        owner: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        shape: &crate::domain::ordered_existence::types::ResolvedShape,
        source: &crate::application::file_dag::relations::PublishedInput,
        rows: crate::domain::operator::contract::views::BorrowedRows<'_>,
        output_kinds: &[ryo_relations::domain::relations::types::CellKind],
        context: &mut OperatorContext<'_>,
        room: &mut Room,
    ) -> Result<crate::application::file_dag::ordered_existence::StrategyOutcome, OperatingError>
    {
        Err(OperatingError::NotRead)
    }

    fn derived_port(&self) -> std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort> {
        std::sync::Arc::clone(&self.0)
    }
    fn stream(
        &self,
        _: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        _: &OperatorContract,
        _: &mut dyn Feed<'_>,
        _: &mut OperatorContext<'_>,
        _: &mut Room,
        _: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        Err(OperatingError::NotRead)
    }
    fn collect(
        &self,
        _: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        _: &OperatorContract,
        _: &[Rows<'_>],
        _: &mut OperatorContext<'_>,
        _: &mut Table,
    ) -> Result<(), OperatingError> {
        Err(OperatingError::NotRead)
    }
}
#[test]
fn final_unread_or_refusing_consumer_releases_zero_budget_global_index_without_reading_sources() {
    for refuse in [false, true] {
        let plans = [plan(false), plan(true)];
        let request = RequestOperators::prepare(
            &plans,
            &[vec![], vec![]],
            &[],
            crate::outbound::derived::factory::test_port(),
        )
        .unwrap();
        let graph = Arc::new(compile_prepared(&plans, &request));
        let roots = [graph.cuts[0][2].unwrap(), graph.cuts[1][2].unwrap()];
        let dag = std::cell::RefCell::new(FileDagExecution::new(
            graph,
            |_| true,
            &[Relation::LanguageTarget],
        ));
        dag.borrow_mut().limit = 0;
        let owner = dag.relation_owner();
        let engine = crate::application::file_dag::execution::CommonOperatorEngine::new(
            crate::outbound::operators::factory::test_port(),
            owner.derived.semantic_port(),
        );
        let executor = crate::application::file_dag::executor::ScopedExecutor::new(
            &engine,
            std::rc::Rc::clone(&owner),
        )
        .unwrap();
        owner.derived.prepare_file(
            true,
            &[Relation::LanguageTarget],
            &[],
            &[],
            &plans,
            &crate::domain::operator::types::PreparedPool::default(),
            |_| true,
        );
        owner.derived.set_optional_limit(0).unwrap();
        let columns = [
            FactColumn(vec![to_bits(Value::File(FileId(1))).0; 2]),
            FactColumn(vec![to_bits(number(2)).0, to_bits(number(3)).0]),
            FactColumn(vec![to_bits(number(9)).0, to_bits(number(7)).0]),
        ];
        let reads = Cell::new(0);
        let read = |relation| {
            assert_eq!(relation, Relation::LanguageTarget);
            reads.set(reads.get() + 1);
            Ok(Rows::Stored(ColumnsView {
                kinds: kinds(relation),
                columns: &columns,
            }))
        };
        let (mut symbols, mut scratch, mut memo) =
            (Symbols::default(), OperatorScratch::default(), vec![]);
        let mut context = OperatorContext {
            semantic: None,
            prepared_kinds: None,
            prepared_matchers: None,
            bindings: &[],
            symbols: (&mut symbols).into(),
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
            memo: &mut memo,
            scratch: &mut scratch,
        };
        dag.begin_rule(0).unwrap();
        let first = dag
            .demand(roots[0], &read, &executor, &mut context)
            .unwrap();
        let rows = owner.resolve(first.as_ref(), &read).unwrap();
        let mut output = Table::default();
        crate::domain::table_model::service::materialize(&rows, &mut output, &mut vec![]);
        assert_eq!(output.cells, [2, 9, 3, 7].map(number));
        drop(first);
        dag.borrow_mut().finish_rule(0);
        let weak = owner
            .derived
            .view_weak(ryo_derived::domain::contract::types::DerivedKind::Equality)
            .unwrap();
        assert!(
            weak.upgrade().is_some(),
            "future global consumer owns the view after rule-local release"
        );
        assert!(owner.derived.working_bytes() > 0);
        assert_eq!(owner.derived.retained_bytes(), 0);
        let before = reads.get();
        dag.begin_rule(1).unwrap();
        if refuse {
            let refusal = Refuse(owner.derived.semantic_port());
            let refused = crate::application::file_dag::executor::ScopedExecutor::new(
                &refusal,
                std::rc::Rc::clone(&owner),
            )
            .unwrap();
            let result = dag.demand(roots[1], &read, &refused, &mut context);
            assert!(matches!(result, Err(OperatingError::NotRead)));
        }
        dag.borrow_mut().finish_rule(1);
        assert_eq!(
            reads.get(),
            before,
            "cancellation/refusal does not demand the skipped producer"
        );
        assert!(
            weak.upgrade().is_none(),
            "last global consumer releases the unretained physical index"
        );
        assert_eq!(owner.derived.working_bytes(), 0);
        assert_eq!(owner.derived.retained_bytes(), 0);
        assert_eq!(owner.derived.counts().0, 1);
        assert!(dag.borrow().ready.iter().all(Option::is_none));
    }
}
