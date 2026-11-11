use super::program::{corpus, engine, prepared};
use super::subscriber::{Access, Activation, Sample, Window};
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution_runtime::outbound::facts::fixtures::activation_project::NoFacts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::stored_facts::types::Bucket;

/// The public project entry for the declared roots in `demanded` (ascending), observed from
/// phase opening to the last delivery. `intrusion` runs inside the observed window and stands
/// for work no production counter reports; the allocator still sees what it allocates.
pub(crate) fn measure_project(
    total: usize,
    demanded: &[usize],
    intrusion: &dyn Fn(usize),
) -> (Sample, Access, Window) {
    let engine = engine();
    let prepared = prepared(&engine, total);
    let corpus = corpus();
    let covered = [Covered {
        file: FileId(0),
        rules: demanded.iter().copied().map(RuleIndex).collect(),
        partial: vec![],
    }];
    let observed = Activation::default();
    let dispatch = tracing::Dispatch::new(observed.clone());
    let project_symbols = Symbols::default();
    // The first cycle warms callsite registration and allocator metadata; the second
    // sees exactly the same real request and declared roots.
    // The per-rule inputs of the phase are built before the measured cycle: they are the caller's, not activation's.
    let first = vec![Bucket(0); total];
    let inapplicable = vec![
            ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles::default();
            total
        ];
    let refused = vec![
            ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default();
            total
        ];
    let delivered = std::cell::Cell::new(0);
    for _ in 0..2 {
        let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None);
        let partials = Partials::default();
        let mut symbols = Symbols::default();
        tracing::dispatcher::with_default(&dispatch, || {
            tracing::trace!(target: "ryo::diagnostic::activation", event = "begin");
            let phase = request.project_phase().unwrap();
            let rules: Vec<RuleIndex> = demanded.iter().copied().map(RuleIndex).collect();
            let ProjectBatch::Complete(outcomes) = phase.evaluate_rules(
                PhaseRules {
                    rules: &rules,
                    first: &first,
                    corpus: &corpus,
                    covered: &covered,
                    skipped: &[],
                    inapplicable: &inapplicable,
                    refused: &refused,
                    facts: &NoFacts,
                    project: &NoFacts,
                    project_symbols: &project_symbols,
                    arenas: &[],
                },
                &mut symbols,
                |_| Ok::<(), std::convert::Infallible>(()),
                |_| {
                    Ok(PhasePartials {
                        evaluation: partials.clone(),
                        publication: partials.clone(),
                    })
                },
                |_| Ok(()),
                |_, outcome, _, _, _| Ok(outcome),
            ) else {
                panic!("an infallible guard cannot refuse")
            };
            delivered.set(delivered.get() + outcomes.len());
            for outcome in outcomes {
                outcome.unwrap();
            }
            intrusion(total);
            tracing::trace!(target: "ryo::diagnostic::activation", event = "end");
            phase.finish().unwrap();
        });
        request.finish();
    }
    let state = observed.0.lock().unwrap();
    (
        state.sample,
        state.access,
        Window {
            starts: state.starts,
            ends: state.ends,
            active: state.active,
            misordered: state.misordered,
            provider_reads: 0,
            delivered: delivered.get(),
        },
    )
}
