//! The `lint` service: local free-function helpers, private to this use case.

use super::errors::LintError;
use super::types::{
    FeatureFed, LintReport, LintRun, ProjectDependency, ProjectDependencyState, RuleWork,
    SourceProvider, StepFed, Wanting,
};
use crate::application::cross_file::service::CrossFile;
use crate::application::cross_file::types::{
    OrdinaryOutcome, PhaseOutcome, PhasePorts, PhaseRun, Settling,
};
use crate::application::file_analysis::streaming::service::FileVisits;
use crate::application::file_analysis::types::{
    FileVisit, ProjectMaking, RequestVisitSetting, StreamRun,
};
use crate::application::metadata::service::kernel_metadata;
use crate::application::persisting::service::kept;
use crate::application::persisting::types::KeepRun;
use crate::application::storing::errors::StoringError;
use crate::application::storing::service::Storing;
use crate::application::storing::types::RepositoryListing;
use crate::domain::answering::service::settle;
use crate::domain::answering::types::Answered;
use crate::domain::keeping::service::rows_of;
use crate::domain::listing::service::every_file;
use crate::domain::memory_diagnostics::constants::{AFTER_FILES, BEFORE_CROSS_CLEANUP};
use crate::domain::memory_diagnostics::service::{
    observed_arenas, observed_cross_symbols, observed_notes, observed_records,
};
use crate::domain::project::service::{project_rules, reads_any};
use crate::domain::project::types::{PlanReads, Projected};
use crate::domain::render::types::Rendered;
use crate::domain::reporting::service::{read_outcomes, sort_unique};
use crate::domain::reporting::types::RowCounts;
use crate::domain::run_rules::service::{owner_of, scopes_of};
use crate::domain::reporting::service::unknowns_from;
use crate::domain::storing::types::{Decision, ReadAlone, Reused, ToKeep};
use crate::domain::visit::service::wants;
use crate::domain::workers::service::order;
use crate::domain::workers::types::{CrossReading, ProjectHandle, Visited, WorkersMet};
use crate::inbound::use_cases::lint::constants::NAMED_PUBLICATION_WAS_NOT_ADMITTED_BY;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::lint_store::types::StoreKeeping;
use crate::outbound::row_sink::interface::RowSinkPort;
use ryo_execution::inbound::use_cases::execution::interface::{Counting, RowsFed};
use ryo_execution::inbound::use_cases::metadata::interface::{MetadataUseCase as _, PlacedScopes};
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_execution_runtime::outbound::facts::interface::FactsWorker;
use ryo_execution::outbound::matching::interface::MatchingPort;
use ryo_language::inbound::use_cases::project::types::ManifestFile;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::Freshness;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stores::types::{MadeAgain, Reuse, StoredRule};

/// The provider a request names, when it names one.
pub(super) fn named_provider(
    provider: &SourceProvider,
) -> Option<&ryo_values::domain::descriptions::types::LanguageName> {
    match provider {
        SourceProvider::Recognized => None,
        SourceProvider::Named(name) => Some(name),
    }
}

/// The failure of the store's listing as the lint answers it: each port's own.
pub(super) fn from_storing(error: StoringError) -> LintError {
    match error {
        StoringError::Corpus(error) => error.into(),
        StoringError::Store(error) => error.into(),
        StoringError::Memory(error) => error.into(),
        StoringError::Budget(error) => error.into(),
        StoringError::Scheduler(error) => error.into(),
    }
}

/// Which rule governs which file, and which relations the rules read: those the languages are asked for.
pub(super) fn wanting(
    matching: &dyn MatchingPort,
    language: &dyn LanguagePort,
    rules: &[RuleSpec],
    plans: &[Plan],
    ids: &[RuleId],
) -> Result<Wanting, LintError> {
    let placement = kernel_metadata().decide(&scopes_of(rules));
    let matcher = matching
        .compile(&placement.globs)
        .map_err(|error| match error {
            MatchingError::BadGlob { glob, reason } => LintError::Rule {
                id: owner_of(&placement, &Glob(glob.into()), ids)
                    .map(|id| id.0.to_string())
                    .unwrap_or_default(),
                reason,
            },
            MatchingError::InvalidRegex { reason } => LintError::Rule {
                id: String::new(),
                reason,
            },
        })?;
    let wants = wants(plans, &language.relations()?.0)?;
    let placed = PlacedScopes { placement, matcher };
    Ok(Wanting { placed, wants })
}

/// What the run uses again of what the last kept: the changed files are read alone once the files the project's lookup consulted
/// are checked against what the store kept of them.
pub(super) fn reused(
    corpus: &dyn CorpusPort,
    digest: &dyn DigestPort,
    visits: &FileVisits,
    setting: &RequestVisitSetting<'_>,
    listing: &RepositoryListing,
    manifests: &[ManifestFile],
    stored_rules: &[StoredRule],
) -> Result<Reused, LintError> {
    let decided = Storing::decision(setting.answering, listing, stored_rules);
    let consulted = match &decided.decision {
        Decision::Again | Decision::Check(_) if !project_rules(setting.metadata).is_empty() => {
            let stored = &listing.stored;
            crate::application::project_context::service::as_it_was(
                corpus,
                digest,
                &stored.consulted,
                &stored.consulted_absent,
                &stored.project_observations,
            )
        }
        Decision::Again | Decision::Check(_) | Decision::Whole | Decision::Cross => {
            Freshness::AsItWas
        }
    };
    let alone = match &decided.decision {
        Decision::Check(_) => {
            visits.read_alone(setting, &decided.changed, listing.read.as_deref())?
        }
        Decision::Again | Decision::Whole | Decision::Cross => ReadAlone::default(),
    };
    Ok(Storing::reused(
        decided,
        consulted,
        alone,
        listing,
        manifests,
        setting.metadata,
        setting.ids,
    ))
}

/// Answers the run once the files are read: the project and the rules that cross files, then the report, and what the store keeps.
#[expect(
    clippy::too_many_arguments,
    reason = "the answer is made by the visits, the cross-file phase and the ports of the use case it is a stage of, over what the run hands it"
)]
pub(super) fn answer(
    visits: &FileVisits,
    cross_file: &CrossFile,
    ports: PhasePorts<'_>,
    rows: &dyn RowSinkPort,
    keeps: StoreKeeping,
    counting: Counting,
    run: &LintRun<'_>,
    visit: &FileVisit<'_>,
    project: &mut ProjectHandle<Box<dyn FactsWorker>>,
) -> Result<LintReport, LintError> {
    let stream = StreamRun {
        files: &run.listing.files,
        answering: run.answering,
        only: &run.request.files,
        metadata: run.metadata,
    };
    let Visited {
        workers,
        mut met,
        counted,
        total,
    } = visits.visited(&stream, visit)??;
    let fresh = order(&mut met);
    observed_records(AFTER_FILES, &fresh.records);
    observed_arenas(AFTER_FILES, &met.arenas);
    observed_notes(AFTER_FILES, &met.notes);
    let WorkersMet {
        observations,
        mut arenas,
        mut unread,
        unrecorded,
        covered,
        mut unknowns,
        mut skipped,
        mut notes,
        fed,
        ..
    } = met;
    let mut answered = Answered {
        unknowns: Vec::new(),
        later: Rendered::default(),
        fed,
    };
    if matches!(run.again, Reuse::Cross(_)) {
        // What the rules could not decide in the files used again is what the store kept of that.
        unknowns.extend(unknowns_from(&run.listing.stored, run.read_again));
        unknowns.extend(run.alone.iter().cloned());
    }
    if matches!(run.again, Reuse::Cross(_) | Reuse::Again) {
        skipped.extend(run.skipped.iter().cloned());
    }
    let handed = CrossReading {
        observations: &observations,
        arenas: &mut arenas,
        covered: &covered,
        skipped: &skipped,
        unread: &unread,
        workers,
    };
    // The project is made now when every file was read and a rule reads it, and made again when a rule made again reads it.
    let made = match &run.again {
        Reuse::Cross(MadeAgain::These(rules)) => project_rules(run.metadata)
            .iter()
            .any(|rule| rules.binary_search_by_key(&rule.0, |each| each.0).is_ok()),
        Reuse::Again => false,
        Reuse::Whole | Reuse::Cross(MadeAgain::All | MadeAgain::Nothing) => {
            reads_any(run.metadata) == PlanReads::Reads
        }
    };
    let made = if made {
        let making = ProjectMaking {
            files: &run.listing.files,
            session: &run.request.session,
            manifests: run.manifests,
            wanted: &run.wants.wanted,
            needs: run.needs,
            ids: run.ids,
        };
        Some(visits.project(&making, project, &mut notes)??)
    } else {
        None
    };
    let phase = PhaseRun {
        again: &run.again,
        stored: &run.listing.stored,
        ids: run.ids,
        read_again: run.read_again,
        alone: run.alone,
        metadata: run.metadata,
        needs: run.needs,
        placed: run.placed,
        paths: run.paths,
        files: &run.listing.files,
        answering: run.answering,
        program: &run.program,
        services: run.services,
        first_bucket: run.first_bucket,
        budget: run.listing.budget,
    };
    let PhaseOutcome {
        project: now,
        settling,
    } = cross_file.phase(&phase, made, handed, project, &mut answered, ports)?;
    if let Settling::Ordinary(ordinary) = settling {
        settled(ordinary, &mut answered)?;
    }
    let Answered {
        unknowns: mut observation_unknowns,
        mut later,
        fed: all_fed,
    } = answered;
    // The files whose bytes could not be read, before the refused manifests join them: the next run reads them again.
    let failed = read_outcomes(&mut unread, unrecorded, &now.refused);
    unknowns.append(&mut observation_unknowns);
    sort_unique(&mut unknowns);
    sort_unique(&mut skipped);
    let files = RowCounts {
        total,
        shown: counted,
    };
    let summary = visits.write_later(&stream, &mut later, files);
    rows.end(summary, &unread)?;
    // What the run found is kept only once its answer is whole: a run that fails leaves the store as it was.
    let keep = KeepRun {
        answering: run.answering,
        again: &run.again,
        read_again: run.read_again,
        stored: &run.listing.stored,
        metadata: run.metadata,
        first_bucket: run.first_bucket,
        wanted: run.wanted,
        handed: run.handed,
    };
    let keeping = ToKeep {
        rules: run.stored_rules.to_vec(),
        records: every_file(&run.listing.files, fresh, &failed),
        later: &rows_of(&later),
        refused: &now.refused,
        unknowns: &unknowns,
        skipped: &skipped,
        consulted: &now.consulted,
        consulted_absent: &now.consulted_absent,
        project_observations: &now.observed_entries,
    };
    kept(keeps, run.listing.read.as_ref(), &keep, keeping)?;
    Ok(LintReport {
        dependencies: project_dependencies(&now),
        unread,
        unknowns,
        skipped,
        features: features_of(counting, run, &all_fed),
        work: work_of(counting, run, &all_fed),
    })
}

/// Settles the outcomes of the ordinary project phase into the answers, keeping an earlier rule or budget failure over a refusal of
/// the stage's release.
pub(super) fn settled(ordinary: OrdinaryOutcome, answered: &mut Answered) -> Result<(), LintError> {
    let OrdinaryOutcome {
        outcomes,
        refused,
        finished,
        mut symbols,
    } = ordinary;
    observed_cross_symbols(BEFORE_CROSS_CLEANUP, &mut symbols);
    for outcome in outcomes {
        if settle(
            &outcome.todo,
            outcome.result.map_err(LintError::from)?.crossed,
            &mut answered.later,
            &mut answered.fed,
            &mut answered.unknowns,
        )
        .is_some()
        {
            return Err(LintError::ObservationBindings {
                reason: NAMED_PUBLICATION_WAS_NOT_ADMITTED_BY.into(),
            });
        }
    }
    if let Some(error) = refused {
        return Err(error.into());
    }
    finished.map_err(LintError::from)
}

/// What the report says each feature of the rules was fed, when the run counts the rows.
fn features_of(counting: Counting, run: &LintRun<'_>, fed: &[Vec<RowsFed>]) -> Vec<FeatureFed> {
    match counting {
        Counting::Counts => kernel_metadata()
            .fed_over(run.metadata, fed)
            .into_iter()
            .map(FeatureFed::from)
            .collect(),
        Counting::DoesNotCount => Vec::new(),
    }
}

/// What the report says each step of each rule was fed, when the run counts the rows.
fn work_of(counting: Counting, run: &LintRun<'_>, fed: &[Vec<RowsFed>]) -> Vec<RuleWork> {
    match counting {
        Counting::Counts => kernel_metadata()
            .work_over(run.metadata, fed)
            .into_iter()
            .zip(run.rules)
            .map(|(steps, rule)| RuleWork {
                rule: RuleId(rule.id.0.clone()),
                steps: steps
                    .into_iter()
                    .map(|each| StepFed {
                        operation: each.operation,
                        rows: each.rows,
                    })
                    .collect(),
            })
            .collect(),
        Counting::DoesNotCount => Vec::new(),
    }
}

/// Preserve present and absent project dependencies in the completed report.
fn project_dependencies(now: &Projected) -> Vec<ProjectDependency> {
    now.consulted
        .iter()
        .map(|dependency| ProjectDependency {
            path: dependency.path.clone(),
            state: ProjectDependencyState::Present(dependency.digest),
        })
        .chain(now.consulted_absent.iter().map(|path| ProjectDependency {
            path: path.clone(),
            state: ProjectDependencyState::Missing,
        }))
        .collect()
}
