//! One source pass and one frontier decode per population across the rules of a cohort, and
//! retirement on every exit.
use super::super::super::super::*;
use super::super::fixtures::{PhaseExit, PreparedCase, prepare_case};
use crate::application::file_dag::frontiers::types::DecodeCount;
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::frontiers::types::PackedFrontier;
use crate::application::file_dag::frontiers::types::FrontierSource;
use crate::application::file_dag::frontiers::fixtures::prepared_frontiers;
use crate::application::file_dag::program::facade_runtime::project_execution::ProjectRoot;
use crate::application::file_dag::program::facade_runtime::project_execution::evaluate_roots;
use crate::application::file_dag::program::facade_runtime::types::ProjectFrontiers;
use crate::application::file_dag::program::native_walk::project::service::ProjectWalk;
use crate::application::file_dag::program::native_walk::{Deliveries, RetainedBatches};
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution_domain::domain::shared_walk::types::WalkCounts;
use ryo_execution_domain::domain::{
    completion::types::CoverageState, coverage::types::Partials, table_model::types::Frontier,
};

/// What the project walk did for one batch of four rules over the given source populations.
struct Observed {
    counts: WalkCounts,
    retained: RetainedBatches,
    delivered: usize,
    refused: bool,
    decodes: DecodeCount,
}

/// How each rule's source rows reach the project run: decoded already, or packed as the store
/// keeps them, each rule from its own bucket holding its own copy of the words.
#[derive(Clone, Copy)]
enum Stored {
    Decoded,
    Packed,
}

/// The source rows as a rule reads them, decoded or packed.
fn frontier_row(stored: Stored, source: PlanInput, table: Table) -> FrontierInput {
    match stored {
        Stored::Decoded => FrontierInput::Decoded {
            frontier: Frontier {
                input: source,
                table: Arc::new(table),
            },
            coverage: CoverageState::Complete,
        },
        Stored::Packed => {
            let mut words = ryo_relations::domain::relations::types::Words::default();
            ryo_values::domain::table_words::service::encode_table(&table, &[], &mut words);
            FrontierInput::Packed(PackedFrontier {
                input: source,
                words: Arc::new(ryo_values::domain::stored_facts::types::KeptWords(
                    ryo_values::domain::table_words::service::pack(
                        ryo_values::domain::table_words::types::WordSlice(&words.0),
                    ),
                )),
                mode: ryo_values::domain::table_words::types::DecodeMode::Permissive,
                coverage: CoverageState::Complete,
            })
        }
    }
}

/// Each rule reads a two-row source; rules whose `values` entries are equal read one population.
fn exercise(exit: PhaseExit, values: [i64; 4], stored: Stored) -> Observed {
    let PreparedCase {
        prepared,
        services,
        corpus,
    } = prepare_case(exit).expect("compile the four project readers");
    let phase = crate::application::file_dag::program::facade_runtime::service::publication_owner(
        &prepared, &services, std::sync::Arc::new(crate::outbound::digest::fixtures::fake::DigestFake),
    )
    .expect("open the publication scope");
    let required =
        crate::application::file_dag::program::facade_runtime::service::required_partitions(
            &prepared,
            RuleIndex(0),
            &corpus,
            &[],
            &[],
            &[],
            &[],
            &services,
        )
        .expect("declare the required partitions");
    let source = PlanInput::Relation(Relation::LanguageTarget);
    let source_root = prepared.program().rules()[0].frontier[0].clone();
    let partials = Partials::default();
    let mut symbols = Symbols::default();
    let demand = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let roots: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(at, value)| {
            let table = Table {
                kinds: kinds(Relation::LanguageTarget).to_vec(),
                cells: vec![
                    Cell::File(FileId(0)),
                    Cell::Number(Integer(*value)),
                    Cell::Number(Integer(9)),
                    Cell::File(FileId(0)),
                    Cell::Number(Integer(3)),
                    Cell::Number(Integer(7)),
                ],
                count: RowCount(2),
            };
            ProjectRoot {
                rule: RuleIndex(at),
                demand,
                run: crate::application::file_dag::program::facade_runtime::project_execution::open_project(
                    &prepared,
                    required.clone(),
                    ScopePartials::Project {
                        evaluation: &partials,
                        publication: &partials,
                    },
                    ProjectFrontiers {
                        publication: &phase,
                        ordinary: prepared_frontiers(
                            RuleIndex(at),
                            vec![FrontierSource {
                                input: source,
                                root: source_root.clone(),
                            }]
                            .into(),
                            vec![frontier_row(stored, source, table)].into(),
                            Arc::clone(prepared.program()),
                            required.clone(),
                            vec![].into(),
                        ),
                        bound_inputs: &[],
                    },
                    &mut symbols,
                    &[],
                    Arc::clone(&services),
                ),
            }
        })
        .collect();
    let walk = &phase;
    let refuse_cohort_of = if matches!(exit, PhaseExit::RefuseLaterCohort) {
        Some(RuleIndex(2))
    } else {
        None
    };
    let mut passes_before_second_cohort = None;
    let batch = evaluate_roots(roots, &mut symbols, |rule| {
        if rule == RuleIndex(2) {
            passes_before_second_cohort = Some(walk.walk_counts().passes.0);
        }
        if Some(rule) == refuse_cohort_of {
            Err(OperatingError::WrongBinding)
        } else {
            Ok(())
        }
    });
    let (delivered, refused) = match batch {
        ProjectBatch::Complete(completed) => (completed.len(), false),
        ProjectBatch::Refused { completed, .. } => (completed.len(), true),
    };
    if values[0] != values[2] {
        assert_eq!(
            passes_before_second_cohort,
            Some(1),
            "the first population is read once before the second cohort is scheduled"
        );
    }
    let observed = Observed {
        counts: phase.walk_counts(),
        retained: phase.walk.retained(),
        delivered,
        refused,
        decodes: phase.frontier_decodes(),
    };
    phase.finish().expect("finish the publication scope");
    observed
}

#[test]
fn one_population_is_read_once_for_every_rule_of_its_cohort() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 2, 2], Stored::Decoded);
    assert_eq!(seen.delivered, 4, "every rule of the cohort is delivered");
    assert_eq!(seen.counts.passes.0, 1, "one source pass serves four rules");
    assert_eq!(
        seen.counts.source_rows.0, 2,
        "the population's rows are walked once"
    );
    assert_eq!(
        seen.retained,
        RetainedBatches(0),
        "the last consumer retired the batch"
    );
}

#[test]
fn a_cohort_of_a_second_population_reads_its_own_source() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 8, 8], Stored::Decoded);
    assert_eq!(seen.delivered, 4);
    assert_eq!(
        seen.counts.passes.0, 2,
        "two populations are two source passes"
    );
    assert_eq!(
        seen.counts.source_rows.0, 4,
        "each population's rows are walked once, never shared with the other"
    );
    assert_eq!(seen.retained, RetainedBatches(0));
}

#[test]
fn retirement_follows_a_failing_terminal_result() {
    let seen = exercise(PhaseExit::SemanticFailure, [2, 2, 2, 2], Stored::Decoded);
    assert_eq!(seen.delivered, 4);
    assert_eq!(
        seen.counts.passes.0, 1,
        "an error in one rule does not reread the source"
    );
    assert_eq!(
        seen.retained,
        RetainedBatches(0),
        "the error left no batch behind"
    );
}

#[test]
fn retirement_follows_a_cancelled_batch() {
    let seen = exercise(PhaseExit::RefuseLaterCohort, [2, 2, 8, 8], Stored::Decoded);
    assert!(seen.refused, "the guard refused the later cohort");
    assert_eq!(
        seen.delivered, 2,
        "the two rules delivered before the refused cohort keep their terminal deliveries"
    );
    assert_eq!(
        seen.counts.passes.0, 1,
        "the refused cohort never reads its source"
    );
    assert_eq!(
        seen.retained,
        RetainedBatches(0),
        "cancellation retires the earlier cohort's batch and leaves the refused one unread"
    );
}

#[test]
fn one_population_is_decoded_once_for_every_rule_of_its_cohort() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 2, 2], Stored::Packed);
    assert_eq!(seen.delivered, 4, "every rule of the cohort is delivered");
    assert_eq!(
        seen.decodes,
        DecodeCount(1),
        "four rules reading their own copies of one population's words decode them once"
    );
    assert_eq!(seen.counts.passes.0, 1, "the shared decode is one population, read in one pass");
    assert_eq!(seen.retained, RetainedBatches(0));
}

#[test]
fn a_second_population_is_decoded_on_its_own() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 8, 8], Stored::Packed);
    assert_eq!(seen.delivered, 4);
    assert_eq!(
        seen.decodes,
        DecodeCount(2),
        "two populations are two decodes, one for each, never one for each rule"
    );
    assert_eq!(seen.counts.passes.0, 2, "two populations are two source passes");
    assert_eq!(seen.retained, RetainedBatches(0));
}

#[test]
fn decoded_rows_are_never_decoded_again() {
    let seen = exercise(PhaseExit::Complete, [2, 2, 2, 2], Stored::Decoded);
    assert_eq!(seen.decodes, DecodeCount(0), "rows handed decoded are admitted as they are");
}

#[test]
fn a_head_is_admitted_only_for_a_declared_root() {
    let PreparedCase { prepared, .. } = prepare_case(PhaseExit::Complete).expect("compile");
    let class = prepared
        .walk()
        .class_of(RuleIndex(0))
        .and_then(|classes| classes.first().copied().flatten())
        .expect("the project reader's first step is a shared head");
    let walk = ProjectWalk::new();
    assert!(
        matches!(walk.admit(&prepared, class), Err(OperatingError::NotRead)),
        "a head whose rule declared no root is refused by name"
    );
    walk.owe(&prepared, RuleIndex(1), Deliveries(1));
    walk.admit(&prepared, class)
        .expect("a sibling that declared the same class shares it");
    walk.delivered(&prepared, RuleIndex(1));
    assert!(
        matches!(walk.admit(&prepared, class), Err(OperatingError::NotRead)),
        "after its last delivery the declaration is gone"
    );
}
