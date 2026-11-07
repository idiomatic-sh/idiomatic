//! What a run keeps between runs, read beside the repository it lists: the files and what the last run kept, each rule's identity,
//! and what the run uses again of the last.
use super::errors::StoringError;
use super::types::{Decided, Listed, RepositoryListing};
use crate::domain::answering::service::cross_steps;
use crate::domain::answering::types::CrossSteps;
use crate::domain::budget::service::{fit, repository_size};
use crate::domain::project::service::{project_rules, reads_project, with_consulted};
use crate::domain::project::types::PlanReads;
use crate::domain::render::types::Answering;
use crate::domain::repository::service::{by_path, configuration};
use crate::domain::reporting::service::{skipped_from, with_gone};
use crate::domain::storing::service::{decide as reuse_of, made_again, records_from};
use crate::domain::workers::service::with_fresh;
use crate::domain::storing::types::{Decision, ReadAlone, Reused};
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::lint_store::interface::LintStorePort;
use crate::outbound::memory::interface::MemoryPort;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::{JobCount, JobNumber, WorkerCount};
use ryo_execution::outbound::digest::interface::DigestPort;
use ryo_execution::outbound::digest::types::DigestBytes;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::ManifestFile;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::hosted::service::identity as hosted_identity;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::Freshness;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::{Crossing, MadeAgain, Reuse, StoredRule};
use std::sync::{Arc, Mutex, PoisonError};

/// What a run keeps between runs, over the ports that list the files, read the store and run the two listing jobs.
pub(crate) struct Storing {
    /// Lists the files under the root.
    corpus: Arc<dyn CorpusPort>,
    /// Digests what each rule is.
    digest: Arc<dyn DigestPort>,
    /// Keeps what a run found of each file and rule for the next run.
    store: Arc<dyn LintStorePort>,
    /// Says how much memory the machine has, which sizes the lint's budget.
    memory: Arc<dyn MemoryPort>,
    /// Runs the two listing jobs.
    scheduler: Arc<dyn SchedulerPort>,
    /// How many workers the scheduler runs jobs on.
    workers: WorkerCount,
}

impl std::fmt::Debug for Storing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl Storing {
    /// Storing over the ports the use case is built with.
    pub(crate) fn new(
        corpus: Arc<dyn CorpusPort>,
        digest: Arc<dyn DigestPort>,
        store: Arc<dyn LintStorePort>,
        memory: Arc<dyn MemoryPort>,
        scheduler: Arc<dyn SchedulerPort>,
        workers: WorkerCount,
    ) -> Self {
        Self {
            corpus,
            digest,
            store,
            memory,
            scheduler,
            workers,
        }
    }

    /// Runs `work` once for each job below `count`, on the scheduler's workers, and answers what each gave, in the order of the jobs.
    fn spread<T>(
        &self,
        count: JobCount,
        work: impl Fn(JobNumber) -> T + Sync,
    ) -> Result<Vec<T>, StoringError>
    where
        T: Send,
    {
        if count.0 == 0 {
            return Ok(Vec::new());
        }
        let slots: Vec<Mutex<Option<T>>> = std::iter::repeat_with(|| Mutex::new(None))
            .take(usize::try_from(count.0).unwrap_or(usize::MAX))
            .collect();
        let mut workers: Vec<Box<dyn FnMut(JobNumber) + Send + '_>> = Vec::new();
        for _ in 0..self.workers.0.clamp(1, count.0) {
            let (slots, work) = (&slots, &work);
            workers.push(Box::new(move |job: JobNumber| {
                let answer = work(job);
                #[expect(
                    clippy::indexing_slicing,
                    reason = "the scheduler hands out only the jobs below the count the slots were made for; another is a bug that must crash"
                )]
                let slot = &slots[usize::try_from(job.0).unwrap_or(usize::MAX)];
                *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(answer);
            }));
        }
        self.scheduler.run(&mut workers, count)?;
        drop(workers);
        Ok(slots
            .into_iter()
            .map(|slot| answered(slot.into_inner().unwrap_or_else(PoisonError::into_inner)))
            .collect())
    }

    /// The files under the root in the order of their paths, and what the last run kept of them, read while they are listed, when
    /// the run answers findings, which it keeps; an answer of the rules' own columns keeps nothing: what it writes is cut at its limit.
    pub(crate) fn listing(&self, answering: &Answering) -> Result<RepositoryListing, StoringError> {
        // The files are listed while the store of the last run is read: neither waits for the other.
        let both = self.spread(JobCount(2), |job| {
            if job.0 == 0 {
                Listed::Files(
                    self.corpus
                        .inventory()
                        .map(|inventory| inventory.0)
                        .map_err(StoringError::from),
                )
            } else if *answering == Answering::Findings {
                Listed::Stored(self.store.stored().map(Some).map_err(StoringError::from))
            } else {
                Listed::Stored(Ok(None))
            }
        })?;
        let (mut listed, mut stored) = (Ok(Vec::new()), Ok(None));
        for each in both {
            match each {
                Listed::Files(files) => listed = files,
                Listed::Stored(kept) => stored = kept,
            }
        }
        let mut files = listed?;
        let budget = self.memory.budget()?;
        fit(repository_size(&files), budget)?;
        by_path(&mut files);
        let read = stored?;
        Ok(RepositoryListing {
            files,
            stored: read
                .as_ref()
                .map_or_else(ryo_values::domain::stores::types::Stored::default, |read| {
                    read.tables()
                }),
            read,
            budget,
        })
    }

    /// What each rule is, as one digest: its query, its parameters, the files it governs and what the host declared; for a rule that
    /// reads what the project writes, what the manifests resolve to too. A rule whose digest changed is run again everywhere.
    pub(crate) fn stored_rules(
        &self,
        rules: &[RuleSpec],
        plans: &[Plan],
        hosted: &[HostRelation],
        resolved: Digest,
        source_language: Option<&ryo_values::domain::descriptions::types::LanguageName>,
    ) -> Vec<StoredRule> {
        let source_selection = self.digest.digest(DigestBytes(
            &crate::domain::repository::service::source_selection_identity(source_language).0,
        ));
        // Schema identity is request-wide: an old retained row cannot acquire a new catalog shape.
        let schema = self.digest.digest(DigestBytes(
            &crate::domain::repository::service::catalog_schema().0,
        ));
        // What the host declares is digested once: each rule's digest combines its own with it.
        let hosted = self.digest.digest(DigestBytes(&hosted_identity(hosted).0));
        rules
            .iter()
            .zip(plans)
            .map(|(rule, plan)| {
                let own = self.digest.combine(&[
                    self.digest.digest(DigestBytes(
                        &configuration(std::slice::from_ref(rule), &[]).0,
                    )),
                    hosted,
                    schema,
                    source_selection,
                ]);
                let own = if plan.observations.is_empty() {
                    own
                } else {
                    self.digest.combine(&[
                        own,
                        self.digest.digest(DigestBytes(
                            &ryo_values::domain::observations::service::output_identity(
                                &plan.observations,
                            )
                            .0,
                        )),
                    ])
                };
                let reads = reads_project(plan) == PlanReads::Reads;
                StoredRule {
                    rule: RuleId(rule.id.0.clone()),
                    digest: if reads {
                        self.digest.combine(&[own, resolved])
                    } else {
                        own
                    },
                    crossing: if reads || cross_steps(plan) == CrossSteps::Has {
                        Crossing::Across
                    } else {
                        Crossing::Within
                    },
                }
            })
            .collect()
    }

    /// What the run does with what the last kept, and the changed files that decision reads alone before the run.
    pub(crate) fn decision(
        answering: &Answering,
        listing: &RepositoryListing,
        stored_rules: &[StoredRule],
    ) -> Decided {
        let decision = match answering {
            Answering::Findings => reuse_of(&listing.stored, stored_rules, &listing.files),
            // Rows are not stored: a rows request has no kept answer to replay, so by the store's contract it evaluates every file again and answers the rows a cold run answers.
            Answering::Rows(_) => Decision::Whole,
        };
        let changed: Vec<CorpusInput> = match &decision {
            Decision::Check(places) => places
                .iter()
                .filter_map(|place| listing.files.get(place.0).cloned())
                .collect(),
            Decision::Whole | Decision::Again | Decision::Cross => Vec::new(),
        };
        Decided { decision, changed }
    }

    /// What the run uses again of what the last kept: whether the files as they were and the findings across files stand, the records
    /// of the files as the store kept them (with those of the changed files read `alone` before the run, when they stand), what the
    /// rules could not decide in those, and the files whose stored rows are theirs no more, in the order of their paths. `consulted`
    /// says whether the files the project's lookup read are as they were.
    #[expect(
        clippy::too_many_arguments,
        reason = "the decision, the files read alone and the freshness of the lookup are made by three different owners before this"
    )]
    pub(crate) fn reused(
        decided: Decided,
        consulted: Freshness,
        alone: ReadAlone,
        listing: &RepositoryListing,
        manifests: &[ManifestFile],
        metadata: &[ryo_values::domain::run_metadata::types::RuleRunMetadata],
        ids: &[RuleId],
    ) -> Reused {
        let Decided { decision, changed } = decided;
        let project = project_rules(metadata);
        let (reuse, alone) = match decision {
            Decision::Whole => (Reuse::Whole, ReadAlone::default()),
            Decision::Again => match with_consulted(MadeAgain::Nothing, &project, consulted) {
                MadeAgain::Nothing => (Reuse::Again, ReadAlone::default()),
                rules @ (MadeAgain::All | MadeAgain::These(_)) => {
                    (Reuse::Cross(rules), ReadAlone::default())
                }
            },
            Decision::Cross => (Reuse::Cross(MadeAgain::All), ReadAlone::default()),
            Decision::Check(_) => {
                let manifests: Vec<FilePath> =
                    manifests.iter().map(|each| each.path.clone()).collect();
                match with_consulted(
                    made_again(&listing.stored, &alone.records, &manifests, &project, ids),
                    &project,
                    consulted,
                ) {
                    MadeAgain::Nothing => (Reuse::Again, alone),
                    rules @ (MadeAgain::All | MadeAgain::These(_)) => (Reuse::Cross(rules), alone),
                }
            }
        };
        let stored = match reuse {
            Reuse::Again | Reuse::Cross(_) => records_from(&listing.stored, ids),
            Reuse::Whole => None,
        };
        // A changed file read before the run is not read again: what the rules found of it then, and what it handed the steps that
        // read across files, are what they are now.
        let (again, records, unknowns) = match stored {
            Some(stored) => (reuse, with_fresh(stored, alone.records), alone.unknowns),
            None => (Reuse::Whole, Vec::new(), Vec::new()),
        };
        let read_again: Vec<FilePath> = listing
            .files
            .iter()
            .filter(|file| {
                let changed = changed
                    .binary_search_by(|each| input_path(each).cmp(input_path(file)))
                    .is_ok();
                let record = match records.binary_search_by(|each| each.path.cmp(input_path(file)))
                {
                    Ok(at) => records.get(at),
                    // The search answers where the path would go: the file has no record.
                    Err(_) => None,
                };
                changed
                    || record.is_none_or(|record| match file {
                        CorpusInput::Available(entry) => {
                            ryo_values::domain::records::service::is_as_it_was(record, entry)
                                != Freshness::AsItWas
                        }
                        CorpusInput::ReadFailed { .. } | CorpusInput::Unreadable { .. } => true,
                    })
            })
            .map(|file| input_path(file).clone())
            .collect();
        // A file the store kept and the run no longer lists is gone: what the store kept of it is not an answer about this run.
        let listed: Vec<&FilePath> = listing.files.iter().map(input_path).collect();
        let read_again = with_gone(read_again, &listing.stored, &listed);
        Reused {
            again,
            records,
            unknowns,
            skipped: skipped_from(&listing.stored, &read_again)
                .into_iter()
                .chain(alone.skipped.iter().cloned())
                .collect(),
            read_again,
        }
    }
}

/// When no worker answered the task: the workers take tasks until none is left, so a slot left empty is a bug that must crash,
/// never a task quietly dropped from the answer.
#[expect(
    clippy::expect_used,
    reason = "the workers take tasks until none is left; a slot left empty is a bug that must crash, never a task dropped from the answer"
)]
fn answered<T>(slot: Option<T>) -> T {
    slot.expect(super::constants::TASK_ANSWERED)
}
