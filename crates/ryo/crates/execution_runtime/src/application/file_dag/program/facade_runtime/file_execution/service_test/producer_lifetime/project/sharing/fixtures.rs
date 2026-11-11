//! The support of the `sharing` cases.
use super::super::fixtures::{PhaseExit, PreparedCase, prepare_case};
use crate::application::file_dag::frontiers::fixtures::prepared_frontiers;
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::frontiers::types::FrontierSource;
use crate::application::file_dag::frontiers::types::PackedFrontier;
use crate::application::file_dag::program::facade_runtime::project_execution::service::ProjectRoot;
use crate::application::file_dag::program::facade_runtime::project_execution::service::evaluate_roots;
use crate::application::file_dag::program::facade_runtime::service::ProjectFrontiers;
use crate::application::file_dag::program::native_walk::service::project::service::ProjectWalk;
use crate::application::file_dag::program::native_walk::types::Deliveries;
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::{
    completion::types::CoverageState, coverage::types::Partials, table_model::types::Frontier,
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{FileId, Integer, Relation, Value as Cell},
};
use ryo_values::domain::{
    plan_parts::types::Input as PlanInput,
    rows::types::{RowCount, Table},
};
use std::sync::Arc;

/// What the project walk did for one batch of four rules over the given source populations: the
/// rules delivered, whether the batch was refused, its source passes (and those made before the
/// second cohort was scheduled, when there is one), the source rows walked, the batches still
/// retained, and the frontier decodes.
#[derive(Debug, PartialEq)]
pub(super) struct Observed {
    pub(super) delivered: usize,
    pub(super) refused: bool,
    pub(super) passes: u64,
    pub(super) passes_before_second_cohort: Option<u64>,
    pub(super) source_rows: u64,
    pub(super) retained: usize,
    pub(super) decodes: usize,
}

/// How each rule's source rows reach the project run: decoded already, or packed as the store
/// keeps them, each rule from its own bucket holding its own copy of the words.
#[derive(Clone, Copy, Debug)]
pub(super) enum Stored {
    Decoded,
    Packed,
}

/// The source rows as a rule reads them, decoded or packed.
pub(super) fn frontier_row(stored: Stored, source: PlanInput, table: Table) -> FrontierInput {
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
pub(super) fn exercise(exit: PhaseExit, values: [i64; 4], stored: Stored) -> Observed {
    let PreparedCase {
        prepared,
        services,
        corpus,
    } = prepare_case(exit).expect("compile the four project readers");
    let phase = crate::application::file_dag::program::facade_runtime::service::publication_owner(
        &prepared,
        &services,
        Arc::new(crate::outbound::digest::fixtures::fake::DigestFake),
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
                run: crate::application::file_dag::program::facade_runtime::project_execution::service::open_project(
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
    let counts = phase.walk_counts();
    let observed = Observed {
        delivered,
        refused,
        passes: counts.passes.0,
        passes_before_second_cohort: passes_before_second_cohort.filter(|_| values[0] != values[2]),
        source_rows: counts.source_rows.0,
        retained: phase.walk.retained().0,
        decodes: phase.frontier_decodes().0,
    };
    phase.finish().expect("finish the publication scope");
    observed
}

/// What the project walk answered when asked to admit the reader's first shared head: before any
/// rule declared it, while a sibling owes a delivery of it, and after that sibling's last delivery.
#[derive(Debug, PartialEq)]
pub(super) struct Admissions {
    pub(super) undeclared: Option<OperatingError>,
    pub(super) declared: Option<OperatingError>,
    pub(super) delivered: Option<OperatingError>,
}

/// Admits the project reader's first shared head through a fresh walk around one declaration.
pub(super) fn admissions() -> Result<Admissions, OperatingError> {
    let PreparedCase { prepared, .. } = prepare_case(PhaseExit::Complete)?;
    let class = prepared
        .walk()
        .class_of(RuleIndex(0))
        .and_then(|classes| classes.first().copied().flatten())
        .ok_or(OperatingError::NotRun)?;
    let walk = ProjectWalk::new();
    let undeclared = walk.admit(&prepared, class).err();
    walk.owe(&prepared, RuleIndex(1), Deliveries(1));
    let declared = walk.admit(&prepared, class).err();
    walk.delivered(&prepared, RuleIndex(1));
    Ok(Admissions {
        undeclared,
        declared,
        delivered: walk.admit(&prepared, class).err(),
    })
}
