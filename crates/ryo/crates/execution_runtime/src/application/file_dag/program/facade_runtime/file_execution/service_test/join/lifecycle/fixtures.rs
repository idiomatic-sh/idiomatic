//! The support of the `lifecycle` cases.
use std::borrow::Cow;
use super::super::super::super::service::*;
use super::super::super::super::*;
use super::super::cases::fixtures::table_of;
use super::super::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::admission::service::Inputs;
use crate::application::file_dag::program::facade_runtime::file_execution::service_test::fixtures::*;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
#[cfg(test)]
use crate::application::file_dag::program::facade_runtime::source_admission::service::SourceInputs;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
#[cfg(test)]
use fixtures::execution::*;
#[cfg(test)]
use fixtures::provider::*;
use rustc_hash::FxHashMap;
#[cfg(test)]
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::Memo;
#[cfg(test)]
use ryo_execution_domain::domain::operator::types::OperatorContext;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::table_model::service::materialize;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    operator::types::Emit as RowSink,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value as Cell};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, Side, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::Table,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::cell::Cell as Counter;
use std::{rc::Rc, sync::Arc};

pub(super) struct Callback<'a>(
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
pub(super) struct DeliveryLookup {
    pub(super) calls: Counter<usize>,
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

/// One delivery attempt of the outer join: the rows delivered before it ended, the frames
/// entered, the nested rows each frame published, and what was kept across frames.
#[derive(Debug, PartialEq)]
pub(super) struct Attempt {
    pub(super) refused: Option<OperatingError>,
    pub(super) delivered: Cow<'static, [Cell]>,
    pub(super) frames: usize,
    pub(super) nested: Cow<'static, [Cell]>,
    /// Frames that borrowed the exact existing lookup and rendered nothing through it.
    pub(super) frames_on_the_lookup: usize,
    /// Frames whose memo held the nested invocation's answer.
    pub(super) frames_holding_their_memo: usize,
    /// Whether the outer memo was restored after the attempt.
    pub(super) memo_restored: bool,
    /// Whether both providers had been read after the attempt.
    pub(super) providers_read: bool,
    /// Whether the attempt read the providers again after the first publication.
    pub(super) reread: bool,
}

/// What a nested join over one owner observed across a refused and a replayed delivery.
#[derive(Debug, PartialEq)]
pub(super) struct Observed {
    pub(super) attempts: [Attempt; 2],
    pub(super) lookup_calls: usize,
    pub(super) reads_unchanged_after_release: bool,
    pub(super) builds: u64,
    pub(super) index_shared: bool,
    pub(super) working_bytes: usize,
}

/// Runs an outer join whose every delivered row invokes a nested join (with a select, extend
/// and project after it when `single_result`) over one owner; the first delivery is refused
/// after one row, the second replays.
pub(super) fn exercise_nested(single_result: bool) -> Result<Observed, OperatingError> {
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
    let kernels =
        super::super::super::super::super::super::kernel_session::service::KernelSession::file(
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
    let mut attempts = Vec::new();
    for fail_after in [Some(1), None] {
        let mut prefix = vec![];
        let mut nested_rows = vec![];
        let mut entries = 0;
        let mut frames_on_the_lookup = 0;
        let mut frames_holding_their_memo = 0;
        let mut nested_observed = FxHashMap::default();
        let mut each = |context: &mut OperatorContext<'_>, row: &[Cell]| {
            entries += 1;
            // The frame borrows the exact existing lookup, without replacing or extending its
            // session.
            let admitted = context.semantic.ok_or(OperatingError::NotRead)?;
            if std::ptr::from_ref(admitted) as *const () == std::ptr::from_ref(&lookup) as *const ()
                && admitted
                    .render_batch(&[], ryo_values::domain::files::types::SourceView(b""))?
                    .is_empty()
            {
                frames_on_the_lookup += 1;
            }
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
            nested_rows.extend(table.cells);
            if *context.memo == [Memo::Holds] {
                frames_holding_their_memo += 1;
            }
            if fail_after == Some(prefix.len()) {
                return Err(OperatingError::NotRead);
            }
            prefix.extend_from_slice(row);
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
        let after_delivery = reads.each_ref().map(|count| count.get());
        // The first committed publications come from both actual providers; retry and nested
        // publication replay do not reread the admitted provider populations.
        let reread = published_reads.is_some_and(|prepared| prepared != after_delivery);
        let providers_read = after_delivery.iter().all(|count| *count > 0);
        if published_reads.is_none() {
            published_reads = Some(after_delivery);
        }
        attempts.push(Attempt {
            refused: result.err(),
            delivered: Cow::Owned(prefix),
            frames: entries,
            nested: Cow::Owned(nested_rows),
            frames_on_the_lookup,
            frames_holding_their_memo,
            memo_restored: *context.memo == [Memo::Fails],
            providers_read,
            reread,
        });
    }
    // Two callbacks before refusal and three on replay.
    let lookup_calls = lookup.calls.get();
    drop(context);
    drop(kernels);
    let reads_unchanged_after_release =
        Some(reads.map(|count| count.get())) == published_reads;
    // One equality index for the actual shared admitted RHS; the nested consumer shares it.
    let builds = owner.derived().counts().0.0;
    let index_shared = owner.derived().counts().1.0 > 0;
    file.finish()?;
    let [first, second]: [Attempt; 2] = attempts
        .try_into()
        .map_err(|_| OperatingError::NotRun)?;
    Ok(Observed {
        attempts: [first, second],
        lookup_calls,
        reads_unchanged_after_release,
        builds,
        index_shared,
        working_bytes: owner.derived().working_bytes().0,
    })
}
