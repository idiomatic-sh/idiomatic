//! Shared production-service controls over declared preparation and the real engine.
#![cfg(test)]
use crate::application::cross_file::{
    errors::CrossFileError,
    preparation::{
        errors::BatchPreparationError,
        service::Preparation,
        types::{OccurrenceNumber, PreparedProject},
    },
    fixtures::{
        ProjectCalls, counting_plan, crossing, frontier_rows, prepared_in, service, todo,
        world_with,
    },
};
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::ReadinessCounts;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::PublicationSlot;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use std::sync::Arc;
/// Observes the production project entry, then delegates to that same engine.
struct CountedProject<'a, T> {
    inner: &'a T,
    submissions: std::cell::Cell<usize>,
}
impl<T: ProjectRootExecution> ProjectRootExecution for CountedProject<'_, T> {
    fn evaluate<E>(
        &self,
        tasks: Vec<ryo_execution::inbound::use_cases::execution::interface::ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<
        Result<
            ryo_execution::inbound::use_cases::execution::interface::RuleOutcome<ryo_execution::inbound::use_cases::execution::interface::ProjectOutputOutcome>,
            ryo_execution::inbound::use_cases::execution::interface::OperatingError,
        >,
        E,
    > {
        self.submissions.set(self.submissions.get() + 1);
        self.inner.evaluate(tasks, symbols, guard)
    }
    fn readiness_counts(&self) -> ReadinessCounts {
        self.inner.readiness_counts()
    }
}
#[derive(Clone, Copy)]
pub(in crate::application::cross_file::preparation) enum BatchControl {
    Singleton,
    Ordinary,
    Named,
    Duplicate,
    PreparationFailure,
    OmitOrdinary,
    OmitNamed,
}

/// Every control reaches CrossFile::evaluate_batch with the real engine stage;
/// an invalid seal must refuse before that stage receives a submission.
pub(in crate::application::cross_file::preparation) fn sealed_batch_control(
    control: BatchControl,
) -> Result<(), BatchPreparationError> {
    use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
    use std::sync::atomic::Ordering;
    let calls = Arc::new(ProjectCalls::default());
    let plan = counting_plan(1, vec![]);
    let rules = world_with(vec![plan.clone(), plan], calls.clone());
    let phase = rules.request.project_phase().unwrap();
    let counted = CountedProject {
        inner: &phase,
        submissions: std::cell::Cell::new(0),
    };
    let mut symbols = Symbols::default();
    let todos = if matches!(control, BatchControl::Singleton) {
        vec![todo(0)]
    } else if matches!(control, BatchControl::Duplicate) {
        vec![todo(0), todo(0), todo(1)]
    } else {
        vec![todo(0), todo(1)]
    };
    let pending = if matches!(control, BatchControl::Named | BatchControl::OmitNamed) {
        Preparation::named(&[RuleIndex(1), RuleIndex(0)], &todos)
    } else {
        Preparation::ordinary(&todos)
    };
    let prepare = |todo: &Todo, symbols: &mut Symbols| {
        let (scope, frontier) = prepared_in(
            &rules,
            &phase,
            todo.rule,
            &rules.covered,
            &frontier_rows(),
            symbols,
        )
        .map_err(CrossFileError::from)?;
        Ok(PreparedProject {
            scope,
            frontier,
            bound: Vec::new(),
            slot: PublicationSlot::Unused,
        })
    };
    if matches!(
        control,
        BatchControl::OmitOrdinary | BatchControl::OmitNamed
    ) {
        let mut pending = pending;
        let first = pending.slots().remove(0);
        let selected = if matches!(control, BatchControl::OmitNamed) {
            &todos[1]
        } else {
            &todos[0]
        };
        pending.account(first, prepare(selected, &mut symbols))?;
        let disposition = match pending.seal() {
            Err(error) => Err(error),
            Ok(batch) => {
                let _ = service().evaluate_batch(
                    &crossing(&rules),
                    batch,
                    &counted,
                    &mut symbols,
                    |_| Ok(()),
                );
                Ok(())
            }
        };
        assert_eq!(
            counted.submissions.get(),
            0,
            "omission must refuse before the actual stage.evaluate call"
        );
        assert_eq!(counted.readiness_counts().evaluations.0, 0);
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 0);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 0);
        phase.finish().unwrap();
        return disposition;
    }
    let sealed = pending
        .prepare_all(|occurrence, todo| {
            if matches!(control, BatchControl::PreparationFailure) && occurrence == OccurrenceNumber(0) {
                Err(CrossFileError::from(OperatingError::WrongBinding))
            } else {
                prepare(todo, &mut symbols)
            }
        })
        .seal()?;
    let expected = sealed.declarations();
    let ProjectBatch::Complete(completed) = service().evaluate_batch(
        &crossing(&rules),
        sealed,
        &counted,
        &mut symbols,
        |_| Ok(()),
    ) else {
        panic!("fixture guard permits the batch")
    };
    assert_eq!(
        counted.submissions.get(),
        1,
        "one complete application inventory makes one engine submission"
    );
    assert_eq!(completed.len(), expected.len());
    for (at, (declaration, outcome)) in expected.iter().zip(completed).enumerate() {
        assert_eq!(
            &outcome.todo, declaration,
            "occurrences preserve declaration identity and order"
        );
        if matches!(control, BatchControl::PreparationFailure) && at == 0 {
            assert!(
                matches!(
                    outcome.result,
                    Err(CrossFileError::Operating(OperatingError::WrongBinding))
                ),
                "explicit preparation failure preserves its actual error"
            );
        } else {
            let result = outcome.result.unwrap();
            assert!(result.crossed.unknowns.is_empty());
            assert!(
                result.crossed.rendered.is_some(),
                "healthy declaration is delivered despite another preparation failure"
            );
        }
    }
    assert_eq!(
        calls.reductions.load(Ordering::SeqCst),
        1,
        "real compatible roots share the engine grouping"
    );
    assert_eq!(counted.readiness_counts().evaluations.0, 1);
    phase.finish().unwrap();
    Ok(())
}
