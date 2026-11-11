//! Reentrant delivery and retry use one genuine prepared program and file owner.
use super::super::super::super::*;
use super::super::cases::test::table_of;
use super::super::*;
use crate::application::file_dag::execution::CommonOperatorEngine;
use ryo_execution_domain::domain::operating::types::Memo;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    operator::types::Emit as RowSink,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, Side, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::Table,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::{rc::Rc, sync::Arc};

struct Callback<'a>(
    &'a mut dyn FnMut(&mut OperatorContext<'_>, &[Cell]) -> Result<(), OperatingError>,
);
impl RowSink for Callback<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Cell],
    ) -> Result<(), OperatingError> {
        self.0(context, row)
    }
}
#[derive(Debug, Default)]
struct DeliveryLookup {
    calls: Counter<usize>,
}
impl ryo_execution_domain::domain::operating::types::SemanticLookup for DeliveryLookup {
    fn render_batch(
        &self,
        requests: &[ryo_language::domain::semantics::types::RenderRequest],
        source: ryo_values::domain::files::types::SourceView<'_>,
    ) -> Result<Vec<ryo_language::domain::semantics::types::RenderOutcome>, OperatingError> {
        assert!(requests.is_empty());
        assert!(source.0.is_empty());
        self.calls.set(self.calls.get() + 1);
        Ok(Vec::new())
    }
}
fn exercise_nested(single_result: bool) -> Result<(), OperatingError> {
    let n = |at| Cell::Number(Integer(at));
    let key = if single_result { 7 } else { 2 };
    let left_table = table_of(&[CellKind::Number], &[&[n(key)]]);
    let right_table = table_of(
        &[CellKind::Number; 2],
        &[
            &[n(key), n(20)],
            &[n(1), n(10)],
            &[n(key), n(21)],
            &[n(key), n(22)],
        ],
    );
    let mut steps = vec![];
    let (left, l) = provider(Rows::Computed(&left_table), 0, &mut steps);
    let right_columns = [
        FactColumn(vec![key as u64, 1, key as u64, key as u64]),
        FactColumn(vec![20, 10, 21, 22]),
    ];
    let (right, r) = provider(
        Rows::Stored(ColumnsView {
            kinds: &[CellKind::Number; 2],
            columns: &right_columns,
        }),
        1,
        &mut steps,
    );
    let request = |output| {
        let mut request = plan(Scenario::ProjectRetry);
        request.steps = steps.clone();
        let mut join = operation(&[0], &[0], &[(Side::Right, output)], vec![], &[]);
        let Operation::Join { left, right, .. } = &mut join else {
            panic!("the fixture builds a Join")
        };
        *left = l;
        *right = r;
        request.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: join,
        });
        request.emit.step = StepId((request.steps.len() - 1) as u32);
        request
    };
    let outer = request(0);
    let mut inner = request(1);
    if single_result {
        let joined = inner.emit.step;
        inner.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Step(joined),
                kept: vec![],
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Literal(QueryLiteral::Number(Integer(20))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
            },
        });
        let selected = StepId((inner.steps.len() - 1) as u32);
        inner.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Extend {
                from: PlanInput::Step(selected),
                values: vec![Operand::Literal(QueryLiteral::Number(Integer(42)))],
            },
        });
        let extended = StepId((inner.steps.len() - 1) as u32);
        inner.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: PlanInput::Step(extended),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            },
        });
        inner.emit.step = StepId((inner.steps.len() - 1) as u32);
    }
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let mut second = rule();
    second.id = RuleId("nested-join".into());
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            plans: vec![outer, inner],
            bindings: vec![vec![], vec![]],
            observations: vec![vec![], vec![]],
            schedule: RuleSchedule::Independent,
        },
        &[rule(), second],
        &[Digest([0; 16]), Digest([1; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(real_port())),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("nested-join.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let providers = [left, right];
    let reads = [Counter::new(0), Counter::new(0)];
    let read = |relation| {
        let side = providers
            .iter()
            .position(|provider| provider.relation == relation)
            .ok_or(OperatingError::NotRead)?;
        reads[side].set(reads[side].get() + 1);
        Ok(providers[side].rows())
    };
    let mut symbols = Symbols::default();
    let lookup = DeliveryLookup::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[
            FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            },
            FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            },
        ],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        Some(&lookup),
        None,
    )?;
    let owner = Rc::clone(&file.owner);
    let outer_root = file
        .program
        .rule_metadata(RuleIndex(0))?
        .emit_root
        .clone()?;
    let inner_root = file
        .program
        .rule_metadata(RuleIndex(1))?
        .emit_root
        .clone()?;
    let kernels = super::super::super::super::super::super::kernel_session::KernelSession::file(
        file.services.kernels(),
        file.program,
        Rc::clone(&owner),
        &read,
        &file.walk,
    );
    let completion = CompletionPolicy::file_inputs(RuleIndex(0), file.file, &[], &[]);
    let inner_completion = CompletionPolicy::file_inputs(RuleIndex(1), file.file, &[], &[]);
    let mut context = file
        .context
        .borrow(&mut symbols, file.program, RuleIndex(0))?;
    context.memo.clear();
    context.memo.push(Memo::Fails);
    // Failure resets only the delivery attempt. The retry uses this exact owner,
    // prepared region, admitted providers, cache and invocation implementation.
    let mut published_reads = None;
    for fail_after in [Some(1), None] {
        let mut prefix = vec![];
        let mut nested_rows = vec![];
        let mut entries = 0;
        let mut nested_observed = FxHashMap::default();
        let mut each = |context: &mut OperatorContext<'_>, row: &[Cell]| {
            entries += 1;
            let admitted = context
                .semantic
                .expect("the same admitted file lookup survives delivery");
            assert_eq!(
                std::ptr::from_ref(admitted) as *const (),
                std::ptr::from_ref(&lookup) as *const (),
                "the frame borrows the exact existing lookup, without replacing or extending its session"
            );
            assert!(
                admitted
                    .render_batch(&[], ryo_values::domain::files::types::SourceView(b""))?
                    .is_empty()
            );
            context.memo.clear();
            context.memo.push(Memo::Holds);
            let mut inputs = SourceInputs {
                execution: &file.execution,
                state: file.execution.file_state(),
                completion: inner_completion.clone(),
                owner: Rc::clone(&owner),
                read: &read,
                observations: &mut nested_observed,
                file_outputs: Some(&file.roots),
                program: file.program,
                rule: RuleIndex(1),
                project: None,
            };
            let InvocationOutcome::Published(input) = file.execution.invoke(
                file.execution.file_state(),
                inner_root,
                &mut inputs,
                &kernels,
                &inner_completion,
                context,
            )?
            else {
                return Err(OperatingError::NotRun);
            };
            let mut table = Table::default();
            materialize(&input.rows(&owner, &read)?, &mut table, &mut vec![]);
            let expected = if single_result {
                vec![n(42)]
            } else {
                vec![n(20), n(21), n(22)]
            };
            assert_eq!(table.cells, expected);
            nested_rows.extend(table.cells.chunks_exact(1).map(<[Cell]>::to_vec));
            assert_eq!(*context.memo, vec![Memo::Holds]);
            if fail_after == Some(prefix.len()) {
                return Err(OperatingError::NotRead);
            }
            prefix.push(row.to_vec());
            Ok(())
        };
        let mut sink = Callback(&mut each);
        let mut inputs = SourceInputs {
            execution: &file.execution,
            state: file.execution.file_state(),
            completion: completion.clone(),
            owner: Rc::clone(&owner),
            read: &read,
            observations: &mut file.observed,
            file_outputs: Some(&file.roots),
            program: file.program,
            rule: RuleIndex(0),
            project: None,
        };
        let result = file
            .execution
            .invoke(
                file.execution.file_state(),
                outer_root,
                &mut inputs,
                &kernels,
                &completion,
                &mut context,
            )
            .and_then(|outcome| {
                let InvocationOutcome::Published(input) = outcome else {
                    return Err(OperatingError::NotRun);
                };
                inputs.replay(&input, &mut context, &mut sink)
            });
        drop(sink);
        drop(each);
        if fail_after.is_some() {
            assert!(matches!(result, Err(OperatingError::NotRead)));
            assert_eq!(prefix, vec![vec![n(key)]]);
            assert_eq!(entries, 2);
        } else {
            result?;
            assert_eq!(prefix, vec![vec![n(key)]; 3]);
            assert_eq!(entries, 3);
            let expected = if single_result {
                vec![vec![n(42)]; 3]
            } else {
                [20, 21, 22]
                    .into_iter()
                    .map(|at| vec![n(at)])
                    .cycle()
                    .take(9)
                    .collect()
            };
            assert_eq!(nested_rows, expected);
        }
        assert_eq!(*context.memo, vec![Memo::Fails]);
        let after_delivery = reads.each_ref().map(|count| count.get());
        if let Some(prepared_reads) = published_reads {
            assert_eq!(
                after_delivery, prepared_reads,
                "retry and nested publication replay do not reread the admitted provider populations"
            );
        } else {
            assert!(
                after_delivery.iter().all(|count| *count > 0),
                "the first committed publications come from both actual providers"
            );
            published_reads = Some(after_delivery);
        }
    }
    assert_eq!(
        lookup.calls.get(),
        5,
        "two callbacks before refusal and three on replay"
    );
    drop(context);
    drop(kernels);
    assert_eq!(
        reads.map(|count| count.get()),
        published_reads.expect("first delivery prepares both roots")
    );
    assert_eq!(
        owner.derived().counts().0,
        1,
        "one equality index for the actual shared admitted RHS"
    );
    assert!(
        owner.derived().counts().1 > 0,
        "nested consumer shares the live index"
    );
    file.finish()?;
    assert_eq!(owner.derived().working_bytes(), 0);
    Ok(())
}
#[test]
fn same_owner_nested_join_preserves_shared_index_bag_order_and_delivery_retry() {
    exercise_nested(false).expect("one compiled owner, nested Join and delivery retry");
}
#[test]
fn nested_output_keeps_consumer_memo_scratch_prefix_and_resume() {
    exercise_nested(true).expect("actual nested output scope");
}
