// ignore: idiomatic.arch/file-length - the lint use case still wires eleven ports through its stages (read, visit, meet, finish): the pure steps are in 14 domain concepts, and what is left is the order the ports are called in, one stage handing its state to the next, which no domain concept can hold because it calls ports.
//! The `lint` use case: compile each rule once, and decide once which files each governs; then for each file, in the worker that
//! reads it: parse it into the worker's own table and its rows into the worker's tables of the store, and run the per-file steps of
//! the rules that govern it on them. Only the words of the rows the rules emit and the cross-file steps read are kept; the workers'
//! tables meet once, the cross-file steps run on the kept words, and the findings are written. What is decided of the rules, the
//! files and the rows is the domain's; this calls it between the calls of the ports.
use super::constants::COMPILED_PLANS;
use super::errors::LintError;
use super::interface::LintUseCase;
use super::service::{answered, handed_of, records_of};
use super::types::{
    CachedPlan, Compiled, Course, FeatureFed, LintReport, LintRequest, LintRun, Listed,
    RepositoryListing, RuleWork, StepFed, Streaming, TaskIndex, Wanting,
};
use crate::application::file_analysis::types::{FileVisit, Worker};

use crate::domain::answering::service::{cross_steps, settle, todo_list};
use crate::domain::answering::types::Todo;
use crate::domain::answering::types::{Answered, CrossSteps, Crossed};
use crate::domain::budget::service::{fit, repository_size, within};

use crate::domain::choosing::service::chosen;
use crate::domain::crossing::types::CrossingInput;
use crate::domain::features::service::{fed_over, work_over};
use crate::domain::frontiers::service::{decode_input, project_input, project_relation};
use crate::domain::hosted::service::lines;
use crate::domain::keeping::service::{rendered_of, rows_of};
use crate::domain::kind_check::service::{first_unknown, kinds_asked, unknown_message};
use crate::domain::lowering::types::Counting;
use crate::domain::operator::types::RowsFed;
use crate::domain::operator::types::TreeSteps;
use crate::domain::ordering::service::{PENDING_LIMIT, admitted, offer, sort_findings};
use crate::domain::ordering::types::{Admittance, Ordered, Turn};
use crate::domain::placement::service::decide;
use crate::domain::placement::types::PlacedScopes;
use crate::domain::reporting::service::report_refused;

use crate::domain::project::service::{project_rules, reads_any, reads_project, with_consulted};
use crate::domain::project::types::{PlanReads, Projected};
use crate::domain::reading::service::{buckets_of, first_buckets, frontier_bucket};

use crate::domain::listing::service::every_file;
use crate::domain::render::constants::{columns_heading, key_heading, path_heading, rule_heading};
use crate::domain::render::service::{lent_row, row_cells, rows_answer};
use crate::domain::render::types::{Answering, Rendered, RowLimit};
use crate::domain::reporting::service::{room_left, totals};
use crate::domain::reporting::types::RowCounts;
use crate::domain::repository::service::{by_path, configuration, paths_of};
use crate::domain::rules::service::{bind, check_ids, ids_of, owner_of, scopes_of};
use crate::domain::rules::types::RuleSpec;
use crate::domain::storing::service::{
    as_kept, decide as reuse_of, later_from, made_again, records_from, skipped_from, stored_of,
    unknowns_from, with_fresh,
};
use crate::domain::storing::types::{
    Decision, HandedAgain, MadeAgain, ReadAlone, Reuse, Reused, ToKeep,
};
use crate::domain::table_model::constants::{column_heading, line_heading};
use crate::domain::table_model::types::Frontier;
use crate::domain::trust::service::{project_unknowns, scoped_needs};
use crate::domain::trust::types::Needs;
use crate::domain::visit::service::wants;
use crate::domain::visit::types::Keeping;

use crate::domain::workers::service::{absorb, gathering, note_slots, order, size_at, tables_for};
use crate::domain::workers::types::{
    CrossReading, FileOrder, FileTotal, FreshNotes, ProjectHandle, Replaying, VisitSetting,
    Visited, WorkersMet,
};
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::PathStat;
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestBytes;
use crate::outbound::facts::interface::FactsPort;
use crate::outbound::facts::interface::FactsWorker;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::language::types::{ManifestReading, ProjectInput, ProjectNotes};
use crate::outbound::lint_store::interface::LintStorePort;
use crate::outbound::lint_store::types::StoreKeeping;
use crate::outbound::matching::errors::MatchingError;
use crate::outbound::matching::interface::MatchingPort;
use crate::outbound::memory::interface::MemoryPort;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort;
use crate::outbound::rows::interface::RowsPort;
use crate::outbound::rows::types::{LentCell, Summary};
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::{JobCount, JobNumber, WorkerCount};
use ryo_language::inbound::use_cases::project::types::{ManifestFile, ManifestText};

use ryo_language::domain::relations::types::WorkerNumber;

use ryo_language::domain::symbols::types::Symbols;

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, FileBytes, Glob};
use ryo_values::domain::hosted::service::declarations;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plan_parts::types::{Heading, InputNumber};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::{Freshness, PlacedRule, Unreadable};
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stores::types::{BucketsWanted, StoredHanded};
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
    /// Compiles each rule's query into the plan the lint evaluates.
    query_language: Arc<dyn QueryLanguagePort>,
    /// Says how much memory the machine has, which sizes the lint's budget.
    memory: Arc<dyn MemoryPort>,
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
    /// The plan compiled so far for each rule id, shared with every use case built over the same: a rule is compiled once
    /// while they live and the relations the host declares are the same.
    compiled: super::types::CompiledPlans,
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
            query_language,
            memory,
            scheduler,
            workers,
            rows,
            digest,
            store,
            compiled: super::types::CompiledPlans::default(),
            unlowered: TreeSteps::Lowered,
            counting: Counting::DoesNotCount,
        }
    }

    /// The same, keeping the plans it compiles in `plans`, which other use cases share.
    #[must_use]
    pub fn compiling_into(mut self, plans: super::types::CompiledPlans) -> Self {
        self.compiled = plans;
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

    /// The plans of the rules, compiled once while this lives, and what each binds its parameters to.
    fn compile_rules(
        &self,
        rules: &[RuleSpec],
        hosted: &[HostRelation],
    ) -> Result<Compiled, LintError> {
        check_ids(rules)?;
        let declared = declarations(hosted);
        let (mut plans, mut bindings) = (Vec::new(), Vec::new());
        for rule in rules {
            let known = self
                .compiled
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .plans
                .get(&rule.id)
                .filter(|cached| cached.query == rule.query && cached.hosted == declared)
                .map(|cached| cached.plan.clone());
            let plan = if let Some(plan) = known {
                plan
            } else {
                let plan = self
                    .query_language
                    .compile(&rule.query, &declared)
                    .map_err(|QueryLanguageError::Refused { error }| LintError::Query {
                        id: rule.id.0.to_string(),
                        error,
                    })?;
                let cached = CachedPlan {
                    query: rule.query.clone(),
                    hosted: declared.clone(),
                    plan: plan.clone(),
                };
                let mut kept = self
                    .compiled
                    .0
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                // Past the cap the plan kept longest is dropped, whichever run it was kept for and whatever this request names: a
                // session that lints under new rule ids again and again stays bounded, and runs sharing the engine lose one plan at
                // a time, never all of theirs at once. A run holds its own plans, so a dropped one is only compiled again later.
                if kept.plans.insert(rule.id.clone(), cached).is_none() {
                    kept.order.push_back(rule.id.clone());
                }
                while kept.plans.len() > COMPILED_PLANS {
                    let Some(oldest) = kept.order.pop_front() else {
                        break;
                    };
                    drop(kept.plans.remove(&oldest));
                }
                drop(kept);
                plan
            };
            bindings.push(bind(&plan, rule)?);
            plans.push(plan);
        }
        Ok(Compiled { plans, bindings })
    }

    /// Which rule governs which file, and which relations the rules read: those the languages are asked for.
    fn wanting(
        &self,
        rules: &[RuleSpec],
        plans: &[Plan],
        ids: &[RuleId],
    ) -> Result<Wanting, LintError> {
        let placement = decide(&scopes_of(rules));
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
    fn drain<I, T>(
        &self,
        workers: WorkerCount,
        tasks: &[I],
        work: impl Fn(TaskIndex, &I) -> T + Sync,
    ) -> Result<Vec<T>, LintError>
    where
        I: Sync,
        T: Send,
    {
        let total = JobCount(u32::try_from(tasks.len()).unwrap_or(u32::MAX));
        let slots: Vec<Mutex<Option<T>>> = std::iter::repeat_with(|| Mutex::new(None))
            .take(tasks.len())
            .collect();
        let next = AtomicUsize::new(0);
        let mut started: Vec<Box<dyn FnMut(JobNumber) + Send + '_>> = Vec::new();
        for _ in 0..workers.0.min(total.0) {
            let (slots, next, work) = (&slots, &next, &work);
            started.push(Box::new(move |_job: JobNumber| {
                loop {
                    let task = next.fetch_add(1, Ordering::Relaxed);
                    let (Some(slot), Some(item)) = (slots.get(task), tasks.get(task)) else {
                        break;
                    };
                    *slot.lock().unwrap_or_else(PoisonError::into_inner) =
                        Some(work(TaskIndex(task), item));
                }
            }));
        }
        let count = JobCount(u32::try_from(started.len()).unwrap_or(u32::MAX));
        self.scheduler.run(&mut started, count)?;
        drop(started);
        Ok(slots
            .into_iter()
            .map(|slot| answered(slot.into_inner().unwrap_or_else(PoisonError::into_inner)))
            .collect())
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
    ) -> Vec<StoredRule> {
        // What the host declares is digested once: each rule's digest combines its own with it.
        let hosted = self
            .digest
            .digest(DigestBytes(&configuration(&[], &lines(hosted)).0));
        rules
            .iter()
            .zip(plans)
            .map(|(rule, plan)| {
                let own = self.digest.combine(&[
                    self.digest.digest(DigestBytes(
                        &configuration(std::slice::from_ref(rule), &[]).0,
                    )),
                    hosted,
                ]);
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
    fn resolution(&self, manifests: &[ManifestFile], plans: &[Plan]) -> Digest {
        if reads_any(plans) == PlanReads::DoesNotRead {
            return Digest([0; 16]);
        }
        let parts: Vec<Digest> = manifests
            .iter()
            .flat_map(|manifest| {
                [
                    self.digest.digest(DigestBytes(manifest.path.0.as_bytes())),
                    self.digest.digest(DigestBytes(manifest.text.0.as_bytes())),
                ]
            })
            .collect();
        self.digest.combine(&parts)
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
        let changed: Vec<CorpusEntry> = match &decision {
            Decision::Check(places) => places
                .iter()
                .filter_map(|place| listing.files.get(place.0).cloned())
                .collect(),
            Decision::Whole | Decision::Again | Decision::Cross => Vec::new(),
        };
        let project = project_rules(setting.plans);
        // What the project comes to may change with files no run lists: those its lookup read are looked at again only when the run
        // uses something again and a rule reads what the project writes.
        let consulted = match &decision {
            Decision::Again | Decision::Check(_) if !project.is_empty() => {
                self.consulted_now(&listing.stored.consulted, &listing.stored.consulted_absent)
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
                let alone = self.read_alone(setting, &changed)?;
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
                    .binary_search_by(|each| each.path.cmp(&file.path))
                    .is_ok();
                let record = match records.binary_search_by(|each| each.path.cmp(&file.path)) {
                    Ok(at) => records.get(at),
                    // The search answers where the path would go: the file has no record.
                    Err(_) => None,
                };
                changed
                    || record.is_none_or(|record| {
                        ryo_values::domain::records::service::is_as_it_was(record, file)
                            != Freshness::AsItWas
                    })
            })
            .map(|file| file.path.clone())
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

    /// Whether the files the project's lookup read beyond the manifests are as the store kept them: each is stat'ed, and read and
    /// digested only when its size or the time it was last written changed. One that is gone, cannot be read or whose digest changed
    /// is changed, and so is a file the lookup asked for and found none of that is there now.
    fn consulted_now(&self, kept: &[StoredFile], absent: &[FilePath]) -> Freshness {
        // A file the lookup found none of stays so; one there now is one the lookup would have found.
        if absent
            .iter()
            .any(|path| !matches!(self.corpus.stat(path), Ok(PathStat::Missing)))
        {
            return Freshness::Changed;
        }
        let mut bytes = FileBytes::default();
        for each in kept {
            let Ok(PathStat::Found(entry)) = self.corpus.stat(&each.path) else {
                return Freshness::Changed;
            };
            if as_kept(each, &entry) == Freshness::AsItWas {
                continue;
            }
            if self.corpus.read(&entry, &mut bytes).is_err()
                || self.digest.digest(DigestBytes(&bytes.0)) != each.digest
            {
                return Freshness::Changed;
            }
        }
        Freshness::AsItWas
    }

    /// What the changed files show other files now, each read alone: the records the visit makes of them, which the findings of the
    /// rules that read across files stand on or fall with.
    fn read_alone(
        &self,
        setting: &VisitSetting<'_>,
        files: &[CorpusEntry],
    ) -> Result<ReadAlone, LintError> {
        let paths = paths_of(files);
        let setting = VisitSetting {
            paths: &paths,
            records: &[],
            ..*setting
        };
        let visit = FileVisit { setting };
        let mut worker = Worker {
            tables: tables_for(
                WorkerNumber(0),
                setting.plans,
                self.unlowered,
                self.counting,
            ),
            facts: self.facts.worker()?,
            failure: None,
        };
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
    ) -> Result<Compiled, LintError> {
        let compiled = self.compile_rules(rules, hosted)?;
        // A word a rule writes for a kind that no linked language has fails the run before a file is read: nothing runs until it is fixed.
        // Rules that write no word for a kind need no vocabulary.
        if compiled
            .plans
            .iter()
            .zip(&compiled.bindings)
            .all(|(plan, bound)| kinds_asked(plan, bound).is_empty())
        {
            return Ok(compiled);
        }
        let vocabularies = self.language.kind_vocabularies();
        if let Some(unknown) = first_unknown(&compiled.plans, &compiled.bindings, &vocabularies) {
            return Err(LintError::UnknownKind {
                id: rules
                    .get(unknown.rule.0)
                    .map(|rule| rule.id.0.to_string())
                    .unwrap_or_default(),
                reason: unknown_message(&unknown.word, &vocabularies).0.into(),
            });
        }
        Ok(compiled)
    }

    /// The manifests of the run, read once for the whole run.
    fn manifests_of(&self, files: &[CorpusEntry]) -> Result<Vec<ManifestFile>, LintError> {
        let mut manifests: Vec<ManifestFile> = Vec::new();
        let mut text = FileBytes::default();
        for file in files
            .iter()
            .filter(|file| self.language.reads_manifest(&file.path) == ManifestReading::Read)
        {
            self.corpus.read(file, &mut text)?;
            manifests.push(ManifestFile {
                path: file.path.clone(),
                text: ManifestText(String::from_utf8_lossy(&text.0).into()),
            });
        }
        Ok(manifests)
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
            states.push(Worker {
                tables: tables_for(
                    WorkerNumber(number),
                    run.plans,
                    self.unlowered,
                    self.counting,
                ),
                facts: self.facts.worker()?,
                failure: None,
            });
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
        let mut met = gathering(run.plans);
        let mut workers = Vec::new();
        for state in states {
            if let Some(error) = state.failure {
                return Err(error.into());
            }
            workers.push(state.facts);
            absorb(&mut met, state.tables, run.plans);
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
        let crossing = CrossingInput {
            plans: run.plans,
            bindings: run.bindings,
            ids: run.ids,
            needs: run.needs,
            paths: &across,
            arenas: &*handed.arenas,
            files: &listing.files,
            covered: handed.covered,
            skipped: handed.skipped,
            project: &now.partials,
            hosted: &run.request.hosted,
            answering: run.answering,
            unknown: &unknown.0,
        };
        let made = self.drain(
            self.workers,
            todo,
            |_, each| -> Result<Crossed, LintError> {
                self.within_budget(listing.budget, || vec![each.id.clone()])?;
                let mut cross = Symbols::default();
                #[expect(
                    clippy::indexing_slicing,
                    reason = "a rule to cross is one of the run's rules, and each has its plan and its first bucket; another is a bug that must crash, not a rule that crosses nothing"
                )]
                let (first, plan) = (run.first_bucket[each.rule.0], &run.plans[each.rule.0]);
                let mut inputs = Vec::new();
                for (place, input) in plan.frontier.iter().enumerate() {
                    // What the project wrote waits in its own handle, not in the buckets of the files.
                    inputs.push(if let Some(relation) = project_relation(*input) {
                        let rows = handle.facts.rows(relation)?;
                        project_input(rows, *input, &handle.symbols, &mut cross)
                    } else {
                        let bucket = frontier_bucket(first, InputNumber(place));
                        let table = decode_input(&kept_facts.kept(bucket)?, &mut cross);
                        Frontier {
                            input: *input,
                            table,
                        }
                    });
                }
                Ok(self
                    .cross_file
                    .evaluate(&crossing, each, &inputs, &mut cross)?)
            },
        )?;
        for (each, crossed) in todo.iter().zip(made) {
            settle(each, crossed?, &mut answered.later, &mut answered.fed);
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
            BucketsWanted::These(rebuilt) => buckets_of(run.plans, run.first_bucket, None)
                .into_iter()
                .filter(|bucket| rebuilt.binary_search(bucket).is_err())
                .collect(),
        };
        let to_store = stored_of(keeping, &listing.stored, last_handed, &rebuilt, unchanged);
        if let Some(read) = &listing.read {
            read.keep(to_store.stored, &to_store.kept)?;
        }
        Ok(())
    }

    /// What the report says each feature of the rules was fed, when the run counts the rows.
    fn features_of(&self, run: &LintRun<'_>, fed: &[Vec<RowsFed>]) -> Vec<FeatureFed> {
        match self.counting {
            Counting::Counts => fed_over(run.plans, fed)
                .into_iter()
                .map(FeatureFed::from)
                .collect(),
            Counting::DoesNotCount => Vec::new(),
        }
    }

    /// What the report says each step of each rule was fed, when the run counts the rows.
    fn work_of(&self, run: &LintRun<'_>, fed: &[Vec<RowsFed>]) -> Vec<RuleWork> {
        match self.counting {
            Counting::Counts => work_over(run.plans, fed)
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
        // The files the rows of the project are about: those the run lists, then the manifests it does not.
        let named: Vec<FilePath> = run
            .listing
            .files
            .iter()
            .map(|file| file.path.clone())
            .chain(now.unlisted.iter().cloned())
            .collect();
        now.unknowns = project_unknowns(&now.partials, &named, run.needs, run.ids);
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
        let project_again = project_rules(run.plans)
            .iter()
            .any(|rule| rules.binary_search_by_key(&rule.0, |each| each.0).is_ok());
        let now = if project_again {
            self.project_now(run, project, notes)?
        } else {
            Projected {
                refused: run.listing.stored.refused.clone(),
                consulted: run.listing.stored.consulted.clone(),
                consulted_absent: run.listing.stored.consulted_absent.clone(),
                ..Projected::default()
            }
        };
        crate::domain::ordering::service::append(
            &mut answered.later,
            &rendered_of(
                &later_from(&run.listing.stored, run.ids, &made_again).unwrap_or_default(),
            ),
        );
        let mut todo = todo_list(run.plans, run.ids);
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
        let WorkersMet {
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
                ..Projected::default()
            }
        } else {
            let mut now = if reads_any(run.plans) == PlanReads::Reads {
                self.project_now(run, project, &mut notes)?
            } else {
                Projected::default()
            };
            unknowns.append(&mut now.unknowns);
            let todo = todo_list(run.plans, run.ids);
            if !todo.is_empty() {
                self.crossed(run, handed, &todo, project, &now, &mut answered)?;
            }
            now
        };
        let Answered {
            mut later,
            fed: all_fed,
        } = answered;
        // The files whose bytes could not be read, before the refused manifests join them: the next run reads them again.
        let mut failed: Vec<FilePath> = unread
            .iter()
            .filter(|each| matches!(each.reason, Unreadable::Failed(_)))
            .map(|each| each.path.clone())
            .collect();
        failed.extend(unrecorded);
        failed.sort_unstable();
        failed.dedup();
        report_refused(&mut unread, &now.refused);
        unknowns.sort();
        unknowns.dedup();
        skipped.sort();
        skipped.dedup();
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
            },
        )?;
        Ok(LintReport {
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
        let rules: Vec<RuleSpec> = request.rules.iter().map(RuleSpec::from).collect();
        let compiling = tracing::info_span!("compile").entered();
        let Compiled { plans, bindings } = self.checked_rules(&rules, &request.hosted)?;
        let ids = ids_of(&rules);
        let Wanting { placed, wants } = self.wanting(&rules, &plans, &ids)?;
        drop(compiling);
        let answering = match request.answer {
            AnswerShape::Findings => Answering::Findings,
            AnswerShape::Rows(limit) => Answering::Rows(rows_answer(&plans, limit)),
        };
        let listing = {
            let _listing = tracing::info_span!("list").entered();
            self.listing(&answering)?
        };
        let paths = paths_of(&listing.files);
        let manifests = self.manifests_of(&listing.files)?;
        let mut project = ProjectHandle {
            symbols: Symbols::default(),
            facts: self.facts.worker()?,
        };
        let first_bucket = first_buckets(&plans);
        let applicability: Vec<_> = rules.iter().map(|rule| rule.applicability).collect();
        let needs: Vec<Needs> = plans.iter().map(scoped_needs).collect();
        let resolved = self.resolution(&manifests, &plans);
        let stored_rules = self.stored_rules(&rules, &plans, &request.hosted, resolved);
        let (nothing_handed, nothing_wanted) =
            (StoredHanded::default(), BucketsWanted::These(Vec::new()));
        let unknown = self.language.unknown_reasons();
        let mut setting = VisitSetting {
            plans: &plans,
            bindings: &bindings,
            wanted: &wants.walked,
            paths: &paths,
            first_bucket: &first_bucket,
            ids: &ids,
            applicability: &applicability,
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
        };
        let Reused {
            again,
            records: kept_records,
            unknowns: alone_unknowns,
            skipped: alone_skipped,
            read_again,
        } = self.reused(&setting, &listing, &manifests, &stored_rules, &ids)?;
        setting.replaying = match again {
            Reuse::Cross(_) => Replaying::FindingsAndHanded,
            Reuse::Whole | Reuse::Again => Replaying::Findings,
        };
        // A run that makes cross-file steps again reads what the files handed them: the buckets of those rules alone.
        let HandedAgain { wanted, handed } =
            handed_of(listing.read.as_ref(), &plans, &first_bucket, &again)?;
        setting.handed = &handed;
        setting.wanted_buckets = &wanted;
        let records = records_of(&listing.files, &kept_records);
        setting.records = &records;
        let run = LintRun {
            request: &request,
            rules: &rules,
            plans: &plans,
            bindings: &bindings,
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
        let visit = FileVisit { setting };
        self.answer(&run, &visit, &mut project)
    }
}
