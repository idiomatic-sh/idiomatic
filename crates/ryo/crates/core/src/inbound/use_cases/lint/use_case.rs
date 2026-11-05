// ignore: idiomatic.arch/file-length - the lint use case still wires eleven ports through its stages (read, visit, meet, finish): the pure steps are in 14 domain concepts, and what is left is the order the ports are called in, one stage handing its state to the next, which no domain concept can hold because it calls ports.
//! The `lint` use case: compile each rule once, and decide once which files each governs; then for each file, in the worker that
//! reads it: parse it into the worker's own table and its rows into the worker's tables of the store, and run the per-file steps of
//! the rules that govern it on them. Only the words of the rows the rules emit and the cross-file steps read are kept; the workers'
//! tables meet once, the cross-file steps run on the kept words, and the findings are written. What is decided of the rules, the
//! files and the rows is the domain's; this calls it between the calls of the ports.
use super::errors::LintError;
use super::interface::LintUseCase;
use super::service::{
    answered, handed_of, memory_enabled, memory_rule, observed_arenas, observed_cross_symbols,
    observed_notes, observed_records, observed_staged, observed_stored, records_of,
};
use super::types::{
    Course, FeatureFed, LintReport, LintRequest, LintRun, Listed, RepositoryListing, RuleWork,
    StepFed, Streaming, TaskIndex, Wanting,
};
use crate::application::file_analysis::types::{FileVisit, Worker};
use ryo_execution::inbound::use_cases::execution::{
    interface::{
        ExecutionCompilation, ExecutionRequest, ExecutionUseCase, PreparedExecution,
        PreparedQueries, PreparedReporting,
    },
    types::{CompiledPlans, CompiledQueries, FrontierDemand, FrontierRequest, PreparationError},
    use_case::{CompilationUseCaseImpl, ExecutionUseCaseImpl},
};

use crate::domain::answering::service::{cross_steps, settle, todo_list};
use crate::domain::answering::types::Todo;
use crate::domain::answering::types::{Answered, CrossSteps, Crossed};
use crate::domain::budget::service::{fit, repository_size, within};

use crate::application::run_coordination::settings::CrossingInput;
use crate::domain::choosing::service::chosen;
use crate::domain::keeping::service::{rendered_of, rows_of};
use crate::domain::ordering::service::{PENDING_LIMIT, admitted, offer, sort_findings};
use crate::domain::ordering::types::{Admittance, Ordered, Turn};
use crate::domain::reporting::service::report_refused;
use ryo_execution::inbound::use_cases::execution::types::Counting;
use ryo_execution::inbound::use_cases::execution::types::RowsFed;
use ryo_execution::inbound::use_cases::execution::types::TreeSteps;
use ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes;
use ryo_values::domain::hosted::service::identity as hosted_identity;

use crate::domain::project::service::{project_rules, reads_any, reads_project, with_consulted};
use crate::domain::project::types::{PlanReads, Projected};
use crate::domain::reading::service::{buckets_of, first_buckets};

use crate::domain::listing::service::every_file;
use crate::domain::render::constants::{column_heading, line_heading};
use crate::domain::render::constants::{columns_heading, key_heading, path_heading, rule_heading};
use crate::domain::render::service::{lent_row, row_cells, rows_answer};
use crate::domain::render::types::{Answering, Rendered, RowLimit};
use crate::domain::reporting::service::{room_left, totals};
use crate::domain::reporting::types::RowCounts;
use crate::domain::repository::service::{by_path, configuration, paths_of};
use crate::domain::rules::service::{ids_of, owner_of, scopes_of};
use crate::domain::storing::service::{
    decide as reuse_of, later_from, made_again, records_from, skipped_from, stored_of,
    unknowns_from, with_fresh,
};
use crate::domain::visit::service::wants;
use crate::domain::visit::types::Keeping;
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_values::domain::rule_requests::types::RuleSpec;
use {
    crate::domain::storing::types::Decision, crate::domain::storing::types::HandedAgain,
    crate::domain::storing::types::ReadAlone, crate::domain::storing::types::Reused,
    crate::domain::storing::types::ToKeep, ryo_values::domain::stores::types::MadeAgain,
    ryo_values::domain::stores::types::Reuse,
};

use crate::application::run_coordination::settings::VisitSetting;
use crate::domain::workers::service::{absorb, gathering, note_slots, order, size_at, tables_for};
use crate::domain::workers::types::{
    CrossReading, FileOrder, FileTotal, FreshNotes, ProjectHandle, Replaying, Visited, WorkersMet,
};
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::language::types::{ManifestReading, ProjectInput, ProjectNotes};
use crate::outbound::lint_store::interface::LintStorePort;
use crate::outbound::lint_store::types::StoreKeeping;
use crate::outbound::memory::interface::MemoryPort;
use crate::outbound::rows::interface::RowsPort;
use crate::outbound::rows::types::{LentCell, Summary};
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::{JobCount, JobNumber, WorkerCount};
use ryo_execution::inbound::use_cases::execution::types::OperatorsPort;
use ryo_execution::inbound::use_cases::execution::types::digest::DigestBytes;
use ryo_execution::inbound::use_cases::execution::types::digest::DigestPort;
use ryo_execution::inbound::use_cases::execution::types::facts::FactsPort;
use ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker;
use ryo_execution::inbound::use_cases::execution::types::matching::MatchingError;
use ryo_execution::inbound::use_cases::execution::types::matching::MatchingPort;
use ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort;
use ryo_language::inbound::use_cases::project::types::ManifestFile;

use ryo_relations::domain::relations::types::WorkerNumber;

use ryo_language::domain::symbols::types::Symbols;

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::{CorpusInput, Glob};
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plan_parts::types::{Heading, InputNumber};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::{Freshness, PlacedRule, Unreadable};
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded};
use ryo_values::domain::stores::types::{Crossing, StoredFile, StoredRule};

use ryo_values::domain::records::types::{RowTotal, RowsTextView};
use ryo_values::domain::runs::types::{AnswerShape, MemoryBytes};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};

/// `lint`, over the ports it calls.
pub struct LintUseCaseImpl {
    /// Reads, parses and evaluates one file in the worker that took its job.
    file_analysis: crate::application::file_analysis::service::FileAnalysis,
    /// Renders the findings of the cross-file steps, reading back the files they name.
    cross_file: crate::application::cross_file::service::CrossFile,
    /// Lists the files under the root and reads each one's bytes.
    corpus: Arc<dyn CorpusPort>,
    /// Holds each worker's rows of a file and keeps the words the cross-file steps read.
    facts: Arc<dyn FactsPort>,
    /// Parses a file into its syntax tree and rows.
    language: Arc<dyn LanguagePort>,
    /// Decides which files a rule's include and exclude globs govern.
    matching: Arc<dyn MatchingPort>,
    /// Owns rule compilation, independent per-request binding and bounded session plans.
    compilation: CompilationUseCaseImpl,
    /// Says how much memory the machine has, which sizes the lint's budget.
    memory: Arc<dyn MemoryPort>,
    /// Pure primitive meanings supplied by the composition edge.
    operators: Arc<dyn OperatorsPort>,
    derived: Arc<dyn ryo_execution::inbound::use_cases::execution::types::DerivedPort>,
    /// Runs the files' jobs on the workers.
    scheduler: Arc<dyn SchedulerPort>,
    /// How many workers the scheduler runs jobs on: the run makes one worker for each.
    workers: WorkerCount,
    /// Takes the findings the rules write.
    rows: Arc<dyn RowsPort>,
    /// Digests the rules and files whose results the store keeps between runs.
    digest: Arc<dyn DigestPort>,
    /// Keeps what a run found of each file and rule for the next run, for the build that made it.
    store: Arc<dyn LintStorePort>,
    /// Whether the steps that read only the tree run as operators and not over the positions of its elements.
    unlowered: TreeSteps,
    /// Whether the rows each step is fed are counted, for the report's `features`: a count that costs a call for every row.
    counting: Counting,
}

impl std::fmt::Debug for LintUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
impl LintUseCaseImpl {
    /// `lint` over the ports the edge injects.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "the use case is composed of the ten ports it calls, each injected by the edge on its own: a struct of them would be a second composition between the edge and the use case"
    )]
    pub fn new(
        file_analysis: crate::application::file_analysis::service::FileAnalysis,
        cross_file: crate::application::cross_file::service::CrossFile,
        corpus: Arc<dyn CorpusPort>,
        facts: Arc<dyn FactsPort>,
        language: Arc<dyn LanguagePort>,
        matching: Arc<dyn MatchingPort>,
        query_language: Arc<dyn QueryLanguagePort>,
        memory: Arc<dyn MemoryPort>,
        operators: Arc<dyn OperatorsPort>,
        derived: Arc<dyn ryo_execution::inbound::use_cases::execution::types::DerivedPort>,
        scheduler: Arc<dyn SchedulerPort>,
        workers: WorkerCount,
        rows: Arc<dyn RowsPort>,
        digest: Arc<dyn DigestPort>,
        store: Arc<dyn LintStorePort>,
    ) -> Self {
        Self {
            file_analysis,
            cross_file,
            corpus,
            facts,
            language,
            matching,
            compilation: CompilationUseCaseImpl::new(query_language),
            memory,
            operators,
            derived,
            scheduler,
            workers,
            rows,
            digest,
            store,
            unlowered: TreeSteps::Lowered,
            counting: Counting::DoesNotCount,
        }
    }

    /// The same, keeping the plans it compiles in `plans`, which other use cases share.
    #[must_use]
    pub fn compiling_into(mut self, plans: CompiledPlans) -> Self {
        self.compilation = self.compilation.compiling_into(plans);
        self
    }

    /// The same, counting the rows each step is fed, for the report's `features`.
    #[must_use]
    pub fn counting(mut self) -> Self {
        self.counting = Counting::Counts;
        self
    }

    /// The same, with the steps that read only the tree run as operators, as the proof that the lowering gives their rows.
    #[must_use]
    pub fn unlowered(mut self) -> Self {
        self.unlowered = TreeSteps::AsOperators;
        self
    }

    /// Refuses when the process holds more than the budget, naming the rules the run was working for.
    fn within_budget(
        &self,
        budget: MemoryBytes,
        plans: impl FnOnce() -> Vec<RuleId>,
    ) -> Result<(), LintError> {
        Ok(within(self.memory.resident_bytes()?, budget, plans)?)
    }

    /// Which rule governs which file, and which relations the rules read: those the languages are asked for.
    fn wanting(
        &self,
        rules: &[RuleSpec],
        plans: &[Plan],
        ids: &[RuleId],
    ) -> Result<Wanting, LintError> {
        let placement = MetadataUseCaseImpl::new().decide(&scopes_of(rules));
        let matcher = self.matching.compile(&placement.globs).map_err(
            |MatchingError::BadGlob { glob, reason }| LintError::Rule {
                id: owner_of(&placement, &Glob(glob.into()), ids)
                    .map(|id| id.0.to_string())
                    .unwrap_or_default(),
                reason,
            },
        )?;
        let wants = wants(plans, &self.language.relations()?.0)?;
        let placed = PlacedScopes { placement, matcher };
        Ok(Wanting { placed, wants })
    }

    /// Runs `work` once for each job below `count`, on the scheduler's workers, and answers what each gave, in the order of the jobs.
    fn spread<T>(
        &self,
        count: JobCount,
        work: impl Fn(JobNumber) -> T + Sync,
    ) -> Result<Vec<T>, LintError>
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

    /// Runs `work` for each of `tasks` on at most `workers` workers, each taking tasks until none is left, and answers what
    /// each gave, in the order of the tasks.
    fn drain<I: Sync, T: Send>(
        &self,
        workers: WorkerCount,
        tasks: &[I],
        work: impl Fn(TaskIndex, &I) -> T + Sync,
    ) -> Result<Vec<T>, LintError> {
        crate::application::job_execution::service::drain(
            self.scheduler.as_ref(),
            workers,
            tasks,
            |index, each| work(TaskIndex(index.0), each),
        )
        .map_err(Into::into)
    }

    /// The files under the root in the order of their paths, and what the last run kept of them, read while they are listed, when
    /// the run answers findings, which it keeps; an answer of the rules' own columns keeps nothing: what it writes is cut at its limit.
    fn listing(&self, answering: &Answering) -> Result<RepositoryListing, LintError> {
        // The files are listed while the store of the last run is read: neither waits for the other.
        let both = self.spread(JobCount(2), |job| {
            if job.0 == 0 {
                Listed::Files(
                    self.corpus
                        .inventory()
                        .map(|inventory| inventory.0)
                        .map_err(LintError::from),
                )
            } else if *answering == Answering::Findings {
                Listed::Stored(self.store.stored().map(Some).map_err(LintError::from))
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
    fn stored_rules(
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
        let node_reuse_policy = (self.file_analysis.execution().node_result_reuse
            == ryo_values::domain::runs::types::NodeResultReuse::Enabled)
            .then(|| self.digest.digest(DigestBytes(b"ryo-node-result-reuse-v1")));
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
                // Stored findings from the two physical backends are not interchangeable.
                let own = if self.file_analysis.kernel_strategy()
                    == ryo_execution::inbound::use_cases::execution::types::KernelStrategy::Column
                {
                    self.digest
                        .combine(&[own, self.digest.digest(DigestBytes(b"column-kernel-v1"))])
                } else {
                    own
                };
                let own =
                    node_reuse_policy.map_or(own, |strategy| self.digest.combine(&[own, strategy]));
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

    /// What the manifests come to, as one digest: each manifest's path and text, so a rule that reads the project is made
    /// again when a manifest changed. Nothing to digest when no rule reads what the project writes.
    fn resolution(
        &self,
        manifests: &[ManifestFile],
        plans: &[Plan],
        session: &ryo_values::domain::runs::types::Session,
    ) -> Result<Digest, LintError> {
        if reads_any(&ryo_values::domain::run_metadata::service::from_plans(
            plans,
        )) == PlanReads::DoesNotRead
        {
            return Ok(Digest([0; 16]));
        }
        crate::application::project_context::service::identity(
            manifests,
            session,
            self.digest.as_ref(),
        )
        .map_err(|error| LintError::Corpus {
            reason: error.to_string(),
        })
    }

    /// What the run uses again of what the last kept: whether the files as they were and the findings across files stand, the records
    /// of the files as the store kept them (with those of the changed files read before the run, when they stand), what the rules
    /// could not decide in those, and the files whose stored rows are theirs no more, in the order of their paths.
    fn reused(
        &self,
        setting: &VisitSetting<'_>,
        listing: &RepositoryListing,
        manifests: &[ManifestFile],
        stored_rules: &[StoredRule],
        ids: &[RuleId],
    ) -> Result<Reused, LintError> {
        let decision = match setting.answering {
            Answering::Findings => reuse_of(&listing.stored, stored_rules, &listing.files),
            Answering::Rows(_) => Decision::Whole,
        };
        let changed: Vec<CorpusInput> = match &decision {
            Decision::Check(places) => places
                .iter()
                .filter_map(|place| listing.files.get(place.0).cloned())
                .collect(),
            Decision::Whole | Decision::Again | Decision::Cross => Vec::new(),
        };
        let project = project_rules(setting.metadata);
        // What the project comes to may change with files no run lists: those its lookup read are looked at again only when the run
        // uses something again and a rule reads what the project writes.
        let consulted = match &decision {
            Decision::Again | Decision::Check(_) if !project.is_empty() => {
                crate::application::project_context::service::as_it_was(
                    self.corpus.as_ref(),
                    self.digest.as_ref(),
                    &listing.stored.consulted,
                    &listing.stored.consulted_absent,
                    &listing.stored.project_observations,
                )
            }
            Decision::Again | Decision::Check(_) | Decision::Whole | Decision::Cross => {
                Freshness::AsItWas
            }
        };
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
                let alone = self.read_alone(setting, &changed, listing.read.as_deref())?;
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
        Ok(Reused {
            again,
            records,
            unknowns,
            skipped: skipped_from(&listing.stored, &read_again)
                .into_iter()
                .chain(alone.skipped.iter().cloned())
                .collect(),
            read_again,
        })
    }

    /// What the changed files show other files now, each read alone: the records the visit makes of them, which the findings of the
    /// rules that read across files stand on or fall with.
    fn read_alone(
        &self,
        setting: &VisitSetting<'_>,
        files: &[CorpusInput],
        store: Option<&dyn crate::outbound::lint_store::interface::ReadStore>,
    ) -> Result<ReadAlone, LintError> {
        let paths = paths_of(files);
        let setting = VisitSetting {
            domain: crate::domain::workers::types::VisitSetting {
                paths: &paths,
                records: &[],
                ..(setting.clone()).domain
            },
            ..setting.clone()
        };
        let visit = FileVisit {
            store,
            setting: setting.clone(),
        };
        let mut worker = Worker::new(
            {
                let mut tables = tables_for(WorkerNumber(0), setting.metadata);
                tables
            },
            self.facts.worker()?,
        );
        for place in 0..files.len() {
            self.file_analysis.visit_file(
                &mut worker,
                &visit,
                files,
                JobNumber(u32::try_from(place).unwrap_or(u32::MAX)),
            );
            worker.tables.rendered = Rendered::default();
        }
        if let Some(error) = worker.failure {
            return Err(error.into());
        }
        Ok(ReadAlone {
            records: worker
                .tables
                .fresh
                .into_iter()
                .map(|each| each.record)
                .collect(),
            unknowns: worker.tables.unknowns,
            skipped: worker.tables.skipped,
        })
    }

    /// Writes the findings of one file to the rows port, those of the files asked for, in the order they were put in.
    fn write_findings(&self, rendered: &Rendered, only: &[FilePath]) -> RowTotal {
        let mut written = 0;
        for cells in chosen(rendered, only) {
            self.rows
                .row(&mut lent_row(RowsTextView(&rendered.text), cells).map(LentCell::from));
            written += 1;
        }
        RowTotal(written)
    }

    /// Writes the first `allowed` rows of one file's rows of the rules' own columns to the rows port, in the order they were put
    /// in, and answers how many it wrote.
    fn write_rows(&self, rendered: &Rendered, allowed: RowLimit) -> RowTotal {
        let mut written = 0;
        for each in rendered.order.iter().take(allowed.0) {
            let cells = row_cells(&rendered.cells, &rendered.ends, each.row);
            self.rows
                .row(&mut lent_row(RowsTextView(&rendered.text), cells).map(LentCell::from));
            written += 1;
        }
        RowTotal(written)
    }

    /// Writes the rows of the cross-file steps after those the files streamed, `files`, and answers what the answer has and shows:
    /// every finding, in the order findings leave in; or every row, of which those the limit leaves room for are written.
    fn write_later(&self, run: &LintRun<'_>, later: &mut Rendered, files: RowCounts) -> Summary {
        match run.answering {
            Answering::Findings => {
                sort_findings(later);
                let findings =
                    RowTotal(files.shown.0 + self.write_findings(later, &run.request.files).0);
                Summary {
                    total: findings,
                    shown: findings,
                }
            }
            Answering::Rows(answer) => {
                let room = room_left(answer.limit, files.shown);
                let _written = self.write_rows(
                    later,
                    RowLimit(usize::try_from(room.0).unwrap_or(usize::MAX)),
                );
                let RowCounts { total, shown } = totals(files, later.total, room);
                Summary { total, shown }
            }
        }
    }

    /// The job of one file: wait to be admitted, visit the file, and write its findings when it is the file's turn, or when the
    /// files before it have left.
    fn process(&self, course: &Course<'_>, worker: &mut Worker, job: JobNumber) {
        let place = FileOrder(usize::try_from(job.0).unwrap_or(usize::MAX));
        let size = size_at(course.files, place);
        let stream = course.streaming;
        {
            let mut ordered = stream
                .ordered
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            while admitted(&ordered, size) == Admittance::Waits {
                ordered = stream
                    .turn
                    .wait(ordered)
                    .unwrap_or_else(PoisonError::into_inner);
            }
            ordered.inflight.0 += size.0;
        }
        self.file_analysis
            .visit_file(worker, course.visit, course.files, job);
        stream
            .ordered
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .inflight
            .0 -= size.0;
        stream.turn.notify_all();
        // The file's turn: its findings leave now, or wait for the files before it.
        let rendered = std::mem::take(&mut worker.tables.rendered);
        let mut ordered = stream
            .ordered
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        // Too much waits for the files before this one: this worker waits with it, and the worker that has the file whose turn it
        // is never waits, so the turn always comes.
        while ordered.held.0 >= PENDING_LIMIT && ordered.next != Turn(place.0) {
            ordered = stream
                .turn
                .wait(ordered)
                .unwrap_or_else(PoisonError::into_inner);
        }
        let mut ready = offer(&mut ordered, Turn(place.0), rendered);
        let mut spare = None;
        while let Some(mut each) = ready {
            let written = match course.visit.setting.answering {
                Answering::Findings => {
                    crate::domain::ordering::service::account(&mut ordered, &each);
                    self.write_findings(&each, course.only).0
                }
                Answering::Rows(_) => {
                    let allowed = crate::domain::ordering::service::allowance(&mut ordered, &each);
                    self.write_rows(&each, allowed).0
                }
            };
            let _counted_before = course.counted.fetch_add(written, Ordering::Relaxed);
            if spare.is_none() {
                crate::domain::ordering::service::clear(&mut each);
                spare = Some(each);
            } else {
                crate::domain::ordering::service::recycle(&mut ordered, each);
            }
            ready = crate::domain::ordering::service::next_ready(&mut ordered);
        }
        worker.tables.rendered = spare.or_else(|| ordered.spares.pop()).unwrap_or_default();
        drop(ordered);
        stream.turn.notify_all();
    }

    /// The rules compiled, refused before any file is read when one writes a kind no linked language has.
    fn checked_rules(
        &self,
        rules: &[RuleSpec],
        hosted: &[HostRelation],
    ) -> Result<CompiledQueries, LintError> {
        let compiled = self.compilation.compile(rules, hosted)?;
        // A word a rule writes for a kind that no linked language has fails the run before a file is read: nothing runs until it is fixed.
        // Rules that write no word for a kind need no vocabulary.
        if compiled
            .plans()
            .iter()
            .zip(compiled.bindings())
            .all(|(plan, bound)| {
                MetadataUseCaseImpl::new()
                    .kinds_asked(plan, bound)
                    .is_empty()
            })
        {
            return Ok(compiled);
        }
        let vocabularies = self.language.kind_vocabularies();
        if let Some(unknown) = MetadataUseCaseImpl::new().first_unknown(
            compiled.plans(),
            compiled.bindings(),
            &vocabularies,
        ) {
            return Err(LintError::UnknownKind {
                id: rules
                    .get(unknown.rule.0)
                    .map(|rule| rule.id.0.to_string())
                    .unwrap_or_default(),
                reason: MetadataUseCaseImpl::new()
                    .unknown_kind_message(&unknown.word, &vocabularies)
                    .0
                    .into(),
            });
        }
        Ok(compiled)
    }

    /// Reads each file once, whatever the rules read of it, and lets the workers meet. Every worker owns its buffers and its
    /// tables, so no job takes a lock; what the workers kept meets once, after the jobs.
    fn visited(
        &self,
        run: &LintRun<'_>,
        visit: &FileVisit<'_>,
    ) -> Result<Visited<Box<dyn FactsWorker>>, LintError> {
        let _reading = tracing::info_span!("read").entered();
        let mut states = Vec::new();
        for number in 0..self.workers.0 {
            states.push(Worker::new(
                {
                    let mut tables = tables_for(WorkerNumber(number), run.metadata);
                    tables
                },
                self.facts.worker()?,
            ));
        }
        let (mut headings, limit) = match run.answering {
            Answering::Findings => (
                vec![
                    Heading(rule_heading().into()),
                    Heading(path_heading().into()),
                    Heading(key_heading().into()),
                    Heading(columns_heading().into()),
                ],
                RowTotal(u64::MAX),
            ),
            Answering::Rows(answer) => (answer.headings.clone(), answer.limit),
        };
        headings.push(Heading(line_heading().into()));
        headings.push(Heading(column_heading().into()));
        self.rows.begin(&headings);
        let streaming = Streaming {
            ordered: Mutex::new(Ordered {
                limit,
                ..Ordered::default()
            }),
            turn: Condvar::new(),
        };
        let counted = AtomicU64::new(0);
        let course = Course {
            files: &run.listing.files,
            visit,
            streaming: &streaming,
            only: &run.request.files,
            counted: &counted,
        };
        let mut jobs: Vec<Box<dyn FnMut(JobNumber) + Send + '_>> = Vec::new();
        for state in &mut states {
            let course = &course;
            jobs.push(Box::new(move |job| self.process(course, state, job)));
        }
        let count = JobCount(u32::try_from(run.listing.files.len()).unwrap_or(u32::MAX));
        self.scheduler.run(&mut jobs, count)?;
        drop(jobs);
        // The workers meet: their tables, the text they made, what they could not read.
        let mut met = gathering(run.metadata);
        let mut workers = Vec::new();
        for state in states {
            if let Some(error) = state.failure {
                return Err(error.into());
            }
            workers.push(state.facts);
            absorb(&mut met, state.tables);
        }
        let counted = RowTotal(counted.load(Ordering::Relaxed));
        let total = match run.answering {
            Answering::Findings => counted,
            Answering::Rows(_) => {
                streaming
                    .ordered
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .total
            }
        };
        Ok(Visited {
            workers,
            met,
            counted,
            total,
        })
    }

    /// What the project writes now, made from its manifests and the notes the files left.
    fn projected(
        &self,
        run: &LintRun<'_>,
        project: &mut ProjectHandle<Box<dyn FactsWorker>>,
        notes: &mut FreshNotes,
    ) -> Result<Projected, LintError> {
        let listing = run.listing;
        // What the languages wrote of each file this run read, in the order of the files: every file is read when the project is made.
        let notes = ProjectNotes(note_slots(notes, FileTotal(listing.files.len())));
        let input = ProjectInput {
            root: &run.request.session.root,
            repository: run.request.session.repository,
            search_scope: &run.request.session.search_scope,
            given_inventory: &run.request.session.given_inventory,
            observed_entries: &Default::default(),
            manifests: run.manifests,
            files: &listing.files,
            notes: &notes,
            wanted: &run.wants.wanted,
        };
        // The project is made on the scheduler's pool, the one the edge built: whatever parallel work a language does
        // inside it runs on that pool's workers, never on a pool of its own.
        let mut projected = None;
        {
            let file_analysis = &self.file_analysis;
            let (input, made) = (&input, &mut projected);
            let mut job: Box<dyn FnMut(JobNumber) + Send + '_> = Box::new(move |_| {
                *made = Some(file_analysis.project_now(
                    input,
                    &mut project.symbols,
                    &mut *project.facts,
                ));
            });
            self.scheduler
                .run(std::slice::from_mut(&mut job), JobCount(1))?;
        }
        projected
            .ok_or(LintError::from(SchedulerError::NoWorkers))?
            .map_err(LintError::from)
    }

    /// Makes the rules that cross files. They are independent of one another: each is made by one worker, with names and tables of
    /// its own, from what the files handed over.
    fn crossed(
        &self,
        run: &LintRun<'_>,
        mut handed: CrossReading<'_, Box<dyn FactsWorker>>,
        todo: &[Todo],
        handle: &ProjectHandle<Box<dyn FactsWorker>>,
        now: &Projected,
        answered: &mut Answered,
    ) -> Result<(), LintError> {
        let _crossing = tracing::info_span!("cross").entered();
        let listing = run.listing;
        let unknown = self.language.unknown_reasons();
        let kept_facts = self
            .facts
            .finish(&mut std::mem::take(&mut handed.workers).into_iter())?;
        // The manifests the rows of the project are about that the run does not list are numbered after the files it lists.
        let across: Vec<FilePath> = run
            .paths
            .iter()
            .cloned()
            .chain(now.unlisted.iter().map(|each| FilePath(each.0.clone())))
            .collect();
        let domain = crate::domain::crossing::types::CrossingInput {
            placed: run.placed,
            observations: handed.observations,
            metadata: run.metadata,
            ids: run.ids,
            needs: run.needs,
            paths: &across,
            files: &listing.files,
            covered: handed.covered,
            skipped: handed.skipped,
            project: &now.partials,
            hosted: &run.request.hosted,
            answering: run.answering,
            unknown: &unknown.0,
        };
        if let Some(batches) = run.program.observation_batches() {
            for each in todo {
                self.within_budget(listing.budget, || vec![each.id.clone()])?;
            }
            return self
                .cross_file
                .observations(
                    crate::application::cross_file::types::ObservationRun {
                        domain,
                        services: run.services,
                        program: std::sync::Arc::clone(&run.program),
                        scheduler: self.scheduler.as_ref(),
                        workers: self.workers,
                        batches,
                        first_bucket: run.first_bucket,
                        facts: kept_facts.as_ref(),
                        project: handle.facts.as_ref(),
                        project_symbols: &handle.symbols,
                        todo,
                    },
                    handed.arenas,
                    answered,
                )
                .map_err(Into::into);
        }
        let crossing = CrossingInput {
            services: run.services,
            program: std::sync::Arc::clone(&run.program),
            arenas: &*handed.arenas,
            domain,
        };
        let made = self.drain(
            self.workers,
            todo,
            |_, each| -> Result<Crossed, LintError> {
                self.within_budget(listing.budget, || vec![each.id.clone()])?;
                let _memory_rule = memory_rule(&each.id, each.rule.0);
                // This scope has the same input/symbol lifetime as the original rule closure.
                let result = (|| -> Result<Crossed, LintError> {
                    let mut cross = Symbols::default();
                    #[expect(
                        clippy::indexing_slicing,
                        reason = "a rule to cross is one of the run's rules, and each has its plan and its first bucket; another is a bug that must crash, not a rule that crosses nothing"
                    )]
                    let first = run.first_bucket[each.rule.0];
                    let scope = run.services.project_scope(each.rule, crossing.domain.files, crossing.domain.covered, crossing.domain.skipped)
                        .map_err(|error| LintError::from(crate::domain::rules::service::failed(&each.id, &error)))?;
                    let inputs = run.services.prepare_frontiers(FrontierRequest {
                        scope: &scope, rule: each.rule, first, facts: kept_facts.as_ref(),
                        project: handle.facts.as_ref(), project_symbols: &handle.symbols,
                        demand: FrontierDemand::Diagnostic,
                    }, &mut cross).map_err(|error| match error {
                        PreparationError::Facts(error) => LintError::from(error),
                        PreparationError::Metadata(error) => LintError::from(crate::domain::rules::service::failed(&each.id, &error)),
                    })?;
                    let result = self.cross_file.evaluate(&crossing, each, inputs, &mut cross);
                    observed_cross_symbols("before_cross_cleanup", &mut cross);
                    Ok(result?)
                })();
                if memory_enabled() {
                    tracing::trace!(target: "ryo::diagnostic::lifecycle_memory", phase = "after_cross_cleanup", owner = "decoded_inputs", caller_inputs_owned = false, success = result.is_ok(), decoded_capacity_bytes = 0_usize);
                }
                result
            },
        )?;
        for (each, crossed) in todo.iter().zip(made) {
            if settle(
                each,
                crossed?,
                &mut answered.later,
                &mut answered.fed,
                &mut answered.unknowns,
            )
            .is_some()
            {
                return Err(LintError::ObservationBindings {
                    reason: "named publication was not admitted by the request owner".into(),
                });
            }
        }
        Ok(())
    }

    /// Keeps what this run found for the next, when it keeps anything and it is not what the store already holds: a row for each
    /// file listed, its findings, and those of the rules that read across files.
    fn kept(&self, run: &LintRun<'_>, keeping: ToKeep<'_>) -> Result<(), LintError> {
        let listing = run.listing;
        if *run.answering != Answering::Findings || self.store.keeps() == StoreKeeping::Discards {
            return Ok(());
        }
        // A run that used everything again and read no file keeps the store as it is.
        if run.again == Reuse::Again && run.read_again.is_empty() {
            return Ok(());
        }
        // What the files handed but their words is made again from the records; the words of the buckets the run did not make
        // again are kept as they are.
        let rebuilt = match &run.again {
            Reuse::Whole | Reuse::Cross(MadeAgain::All | MadeAgain::Nothing) => BucketsWanted::All,
            Reuse::Cross(MadeAgain::These(_)) | Reuse::Again => run.wanted.clone(),
        };
        let loaded;
        let last_handed = match &run.again {
            Reuse::Cross(_) | Reuse::Whole => run.handed,
            Reuse::Again => {
                loaded = listing
                    .read
                    .as_ref()
                    .map(|read| read.handed(&BucketsWanted::These(Vec::new())))
                    .transpose()?
                    .unwrap_or_default();
                &loaded
            }
        };
        let unchanged: Vec<Bucket> = match &rebuilt {
            BucketsWanted::All => Vec::new(),
            BucketsWanted::These(rebuilt) => buckets_of(run.metadata, run.first_bucket, None)
                .into_iter()
                .filter(|bucket| rebuilt.binary_search(bucket).is_err())
                .collect(),
        };
        observed_records("before_store_build", &keeping.records);
        let to_store = stored_of(keeping, &listing.stored, last_handed, &rebuilt, unchanged);
        observed_stored("after_store_build", &to_store.stored.handed);
        observed_staged("after_store_build", &to_store.kept);
        if let Some(read) = &listing.read {
            observed_stored("before_store_write", &to_store.stored.handed);
            observed_staged("before_store_write", &to_store.kept);
            let written = read.keep(to_store.stored, &to_store.kept);
            if memory_enabled() {
                // Ownership moved into the outbound call; this does not claim it freed allocator pages.
                tracing::trace!(target: "ryo::diagnostic::lifecycle_memory", phase = "after_store_write", owner = "store_handed", caller_payload_owned = false, success = written.is_ok());
            }
            written?;
        }
        Ok(())
    }

    /// What the report says each feature of the rules was fed, when the run counts the rows.
    fn features_of(&self, run: &LintRun<'_>, fed: &[Vec<RowsFed>]) -> Vec<FeatureFed> {
        match self.counting {
            Counting::Counts => MetadataUseCaseImpl::new()
                .fed_over(run.metadata, fed)
                .into_iter()
                .map(FeatureFed::from)
                .collect(),
            Counting::DoesNotCount => Vec::new(),
        }
    }

    /// What the report says each step of each rule was fed, when the run counts the rows.
    fn work_of(&self, run: &LintRun<'_>, fed: &[Vec<RowsFed>]) -> Vec<RuleWork> {
        match self.counting {
            Counting::Counts => MetadataUseCaseImpl::new()
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

    /// The project made now, and what the rules that read it could not decide for its partly read relations.
    fn project_now(
        &self,
        run: &LintRun<'_>,
        project: &mut ProjectHandle<Box<dyn FactsWorker>>,
        notes: &mut FreshNotes,
    ) -> Result<Projected, LintError> {
        let mut now = self.projected(run, project, notes)?;
        // A primary manifest that could not be read is not a complete empty project.
        // This affects project facts, not the disk-failure diagnosis of unrelated files.
        for input in &run.listing.files {
            let detail = match input {
                CorpusInput::ReadFailed { reason, .. } | CorpusInput::Unreadable { reason, .. }
                    if self.language.reads_manifest(input_path(input)) == ManifestReading::Read =>
                {
                    &reason.0
                }
                _ => continue,
            };
            now.partials
                .extend(crate::domain::project::service::unread_manifest_partials(
                    &run.wants.wanted,
                    input_path(input),
                    detail,
                ));
        }
        // The files the rows of the project are about: those the run lists, then the manifests it does not.
        let named: Vec<FilePath> = run
            .listing
            .files
            .iter()
            .map(|file| input_path(file).clone())
            .chain(now.unlisted.iter().cloned())
            .collect();
        now.unknowns =
            MetadataUseCaseImpl::new().project_unknowns(&now.partials, &named, run.needs, run.ids);
        Ok(now)
    }

    /// Makes the cross-file steps of `rules` alone again, with the other rules' findings as the store kept them, and the project,
    /// unless a rule made again reads it; answers the project, and what the rules could not decide in it.
    fn made_some_again(
        &self,
        run: &LintRun<'_>,
        rules: &[PlacedRule],
        project: &mut ProjectHandle<Box<dyn FactsWorker>>,
        handed: CrossReading<'_, Box<dyn FactsWorker>>,
        notes: &mut FreshNotes,
        answered: &mut Answered,
    ) -> Result<Projected, LintError> {
        let made_again: Vec<RuleId> = rules
            .iter()
            .filter_map(|rule| {
                run.ids
                    .get(usize::try_from(rule.0).unwrap_or(usize::MAX))
                    .cloned()
            })
            .collect();
        let project_again = project_rules(run.metadata)
            .iter()
            .any(|rule| rules.binary_search_by_key(&rule.0, |each| each.0).is_ok());
        let now = if project_again {
            self.project_now(run, project, notes)?
        } else {
            Projected {
                refused: run.listing.stored.refused.clone(),
                consulted: run.listing.stored.consulted.clone(),
                consulted_absent: run.listing.stored.consulted_absent.clone(),
                observed_entries: run.listing.stored.project_observations.clone(),
                ..Projected::default()
            }
        };
        crate::domain::ordering::service::append(
            &mut answered.later,
            &rendered_of(
                &later_from(&run.listing.stored, run.ids, &made_again).unwrap_or_default(),
            ),
        );
        let mut todo = todo_list(run.metadata, run.ids);
        todo.retain(|each| {
            rules
                .binary_search_by_key(&each.rule.0, |rule| {
                    usize::try_from(rule.0).unwrap_or(usize::MAX)
                })
                .is_ok()
        });
        if !todo.is_empty() {
            self.crossed(run, handed, &todo, project, &now, answered)?;
        }
        Ok(now)
    }

    /// Answers the run once the files are read: the project and the rules that cross files, made now when every file was read and
    /// used again as the store kept them otherwise; then the report, and what the store keeps.
    fn answer(
        &self,
        run: &LintRun<'_>,
        visit: &FileVisit<'_>,
        project: &mut ProjectHandle<Box<dyn FactsWorker>>,
    ) -> Result<LintReport, LintError> {
        let Visited {
            workers,
            mut met,
            counted,
            total,
        } = self.visited(run, visit)?;
        let fresh = order(&mut met);
        observed_records("after_files", &fresh.records);
        observed_arenas("after_files", &met.arenas);
        observed_notes("after_files", &met.notes);
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
            workers,
        };
        let now = if let Reuse::Cross(MadeAgain::These(rules)) = &run.again {
            let mut now =
                self.made_some_again(run, rules, project, handed, &mut notes, &mut answered)?;
            unknowns.append(&mut now.unknowns);
            now
        } else if run.again == Reuse::Again {
            crate::domain::ordering::service::append(
                &mut answered.later,
                &rendered_of(&later_from(&run.listing.stored, run.ids, &[]).unwrap_or_default()),
            );
            unknowns.extend(unknowns_from(&run.listing.stored, run.read_again));
            unknowns.extend(run.alone.iter().cloned());
            Projected {
                refused: run.listing.stored.refused.clone(),
                consulted: run.listing.stored.consulted.clone(),
                consulted_absent: run.listing.stored.consulted_absent.clone(),
                observed_entries: run.listing.stored.project_observations.clone(),
                ..Projected::default()
            }
        } else {
            let mut now = if reads_any(run.metadata) == PlanReads::Reads {
                self.project_now(run, project, &mut notes)?
            } else {
                Projected::default()
            };
            unknowns.append(&mut now.unknowns);
            let todo = todo_list(run.metadata, run.ids);
            if !todo.is_empty() {
                self.crossed(run, handed, &todo, project, &now, &mut answered)?;
            }
            now
        };
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
        let summary = self.write_later(run, &mut later, files);
        self.rows.end(summary, &unread)?;
        // What the run found is kept only once its answer is whole: a run that fails leaves the store as it was.
        self.kept(
            run,
            ToKeep {
                rules: run.stored_rules.to_vec(),
                records: every_file(&run.listing.files, fresh, &failed),
                later: &rows_of(&later),
                refused: &now.refused,
                unknowns: &unknowns,
                skipped: &skipped,
                consulted: &now.consulted,
                consulted_absent: &now.consulted_absent,
                project_observations: &now.observed_entries,
            },
        )?;
        Ok(LintReport {
            dependencies: project_dependencies(&now),
            unread,
            unknowns,
            skipped,
            features: self.features_of(run, &all_fed),
            work: self.work_of(run, &all_fed),
        })
    }
}

impl LintUseCase for LintUseCaseImpl {
    fn lint(&self, request: LintRequest) -> Result<LintReport, LintError> {
        self.file_analysis
            .validate_source_language(request.source_language.as_ref())?;
        let rules: Vec<RuleSpec> = request.rules.iter().map(RuleSpec::from).collect();
        let compiling = tracing::info_span!("compile").entered();
        let compiled = self.checked_rules(&rules, &request.hosted)?;
        let plans = compiled.plans();
        let bindings = compiled.bindings();
        let ids = ids_of(&rules);
        let Wanting { placed, wants } = self.wanting(&rules, &plans, &ids)?;
        drop(compiling);
        let answering = match request.answer {
            AnswerShape::Findings => Answering::Findings,
            AnswerShape::Rows(limit) => Answering::Rows(rows_answer(&plans, limit)),
        };
        let mut listing = {
            let _listing = tracing::info_span!("list").entered();
            self.listing(&answering)?
        };
        let paths = paths_of(&listing.files);
        let manifests = self.file_analysis.read_manifests(&mut listing.files)?;
        let mut project = ProjectHandle {
            symbols: Symbols::default(),
            facts: self.facts.worker()?,
        };
        let first_bucket = first_buckets(&plans);
        let applicability: Vec<_> = rules.iter().map(|rule| rule.applicability).collect();
        let applicability_can_gate = applicability.len() != plans.len()
            || applicability.iter().any(|policy| {
                *policy != ryo_values::domain::runs::types::RuleApplicability::Always
            });
        let needs: Vec<Needs> = plans
            .iter()
            .map(|plan| MetadataUseCaseImpl::new().scoped_needs(plan))
            .collect();
        let resolved = self.resolution(&manifests, &plans, &request.session)?;
        let stored_rules = self.stored_rules(
            &rules,
            &plans,
            &request.hosted,
            resolved,
            request.source_language.as_ref(),
        );
        let identities =
            self.compilation
                .identify(&compiled, self.digest.as_ref(), stored_rules)?;
        let stored_rules = identities.rules;
        let (nothing_handed, nothing_wanted) =
            (ReadHanded::default(), BucketsWanted::These(Vec::new()));
        let unknown = self.language.unknown_reasons();
        let execution =
            ExecutionUseCaseImpl::new(Arc::clone(&self.operators), Arc::clone(&self.derived));
        let rule_digests: Vec<_> = stored_rules.iter().map(|rule| rule.digest).collect();
        let program = Arc::new(
            execution
                .prepare_compiled(
                    compiled,
                    rules.clone(),
                    rule_digests,
                    request.hosted.clone(),
                    identities.observations,
                )
                .map_err(|error| {
                    crate::domain::rules::service::failed(&RuleId("<request>".into()), &error)
                })?,
        );
        let services = execution.begin(&program, &listing.files);
        let metadata = program.metadata();
        let mut setting = VisitSetting {
            program: Arc::clone(&program),
            source_language: request.source_language.as_ref(),
            services: &services,
            domain: crate::domain::workers::types::VisitSetting {
                metadata,
                wanted: &wants.walked,
                paths: &paths,
                first_bucket: &first_bucket,
                ids: &ids,
                applicability: &applicability,
                applicability_can_gate,
                placed: &placed,
                records: &[],
                replaying: Replaying::Findings,
                stored: &listing.stored,
                handed: &nothing_handed,
                wanted_buckets: &nothing_wanted,
                budget: listing.budget,
                noted: wants.noted,
                roled: wants.roled,
                needs: &needs,
                answering: &answering,
                keeping: match self.store.keeps() {
                    StoreKeeping::Keeps => Keeping::Keeps,
                    StoreKeeping::Discards => Keeping::Discards,
                },
                unknown: &unknown.0,
            },
        };
        let Reused {
            again,
            records: kept_records,
            unknowns: alone_unknowns,
            skipped: alone_skipped,
            read_again,
        } = self.reused(&setting, &listing, &manifests, &stored_rules, &ids)?;
        let (again, producer_buckets) =
            program
                .replay_closure(again, &first_bucket)
                .map_err(|error| {
                    crate::domain::rules::service::failed(&RuleId("<request>".into()), &error)
                })?;
        setting.replaying = match again {
            Reuse::Cross(_) => Replaying::FindingsAndHanded,
            Reuse::Whole | Reuse::Again => Replaying::Findings,
        };
        // A run that makes cross-file steps again reads what the files handed them: the buckets of those rules alone.
        let HandedAgain { wanted, handed } = handed_of(
            listing.read.as_ref(),
            metadata,
            &first_bucket,
            &again,
            &producer_buckets,
        )?;
        setting.handed = &handed;
        setting.wanted_buckets = &wanted;
        let records = records_of(&listing.files, &kept_records);
        setting.records = &records;
        let run = LintRun {
            placed: &placed,
            program: Arc::clone(&program),
            services: &services,
            request: &request,
            rules: &rules,
            metadata,
            ids: &ids,
            wants: &wants,
            listing: &listing,
            paths: &paths,
            manifests: &manifests,
            first_bucket: &first_bucket,
            needs: &needs,
            answering: &answering,
            stored_rules: &stored_rules,
            again,
            read_again: &read_again,
            alone: &alone_unknowns,
            skipped: &alone_skipped,
            handed: &handed,
            wanted: &wanted,
        };
        let visit = FileVisit {
            store: listing.read.as_deref(),
            setting,
        };
        self.answer(&run, &visit, &mut project)
    }
}

/// Preserve present and absent project dependencies in the completed report.
fn project_dependencies(now: &Projected) -> Vec<super::types::ProjectDependency> {
    now.consulted
        .iter()
        .map(|dependency| super::types::ProjectDependency {
            path: dependency.path.clone(),
            state: super::types::ProjectDependencyState::Present(dependency.digest),
        })
        .chain(
            now.consulted_absent
                .iter()
                .map(|path| super::types::ProjectDependency {
                    path: path.clone(),
                    state: super::types::ProjectDependencyState::Missing,
                }),
        )
        .collect()
}

/// Collect retry paths before adding refused manifests to the final unread report.
fn read_outcomes(
    unread: &mut Vec<ryo_values::domain::records::types::Unread>,
    unrecorded: Vec<FilePath>,
    refused: &[ryo_values::domain::runs::types::ManifestRefused],
) -> Vec<FilePath> {
    let mut failed: Vec<FilePath> = unread
        .iter()
        .filter(|each| matches!(each.reason, Unreadable::Failed(_)))
        .map(|each| each.path.clone())
        .collect();
    failed.extend(unrecorded);
    failed.sort_unstable();
    failed.dedup();
    report_refused(unread, refused);
    failed
}

/// Preserve one ordered occurrence of each completed coverage record.
fn sort_unique<T>(records: &mut Vec<T>)
where
    T: Ord,
{
    records.sort();
    records.dedup();
}

use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
