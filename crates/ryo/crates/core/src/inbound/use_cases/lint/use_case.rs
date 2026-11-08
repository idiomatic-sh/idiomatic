//! The `lint` use case: compile each rule once, and decide once which files each governs; then for each file, in the worker that
//! reads it: parse it into the worker's own table and its rows into the worker's tables of the store, and run the per-file steps of
//! the rules that govern it on them in the engine's one call. Only the words of the rows the rules emit and the cross-file steps read
//! are kept; the workers' tables meet once, the cross-file steps run on the kept words, and the findings are written. The store's
//! listing and reuse, the visits of the files and the cross-file phase are the application's; this wires them and publishes.
use super::errors::LintError;
use super::interface::LintUseCase;
use super::service::{answer, from_storing, named_provider, reused, wanting};
use super::types::{LintReport, LintRequest, LintRun, Wanting};
use crate::application::cross_file::service::CrossFile;
use crate::application::cross_file::types::PhasePorts;
use crate::application::engine::service::{Compilation, Execution};
use crate::application::file_analysis::service::FileAnalysis;
use crate::application::file_analysis::streaming::service::FileVisits;
use crate::application::file_analysis::types::{FileVisit, RequestVisitSetting};
use crate::application::metadata::service::kernel_metadata;
use crate::application::persisting::service::handed_of;
use crate::application::storing::service::Storing;
use crate::domain::reading::service::first_buckets;
use crate::domain::render::service::rows_answer;
use crate::domain::render::types::Answering;
use crate::domain::repository::service::paths_of;
use crate::domain::run_rules::service::ids_of;
use crate::domain::storing::types::Reused;
use crate::domain::visit::types::Keeping;
use crate::domain::workers::service::{gating_of, records_of};
use crate::domain::workers::types::ProjectHandle;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::language::interface::LanguagePort;
use crate::outbound::lint_store::interface::LintStorePort;
use crate::outbound::lint_store::types::StoreKeeping;
use crate::outbound::memory::interface::MemoryPort;
use crate::outbound::row_sink::interface::RowSinkPort;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::WorkerCount;
use ryo_execution::inbound::use_cases::execution::interface::ReplayClosure;
use ryo_execution::inbound::use_cases::execution::interface::PreparedReporting as _;
use ryo_execution::inbound::use_cases::prepare::interface::{
    CompiledPlans, ExecutionCompilation as _, PreparedQueries as _,
};
use ryo_execution::inbound::use_cases::execution::interface::Counting;
use ryo_execution::inbound::use_cases::metadata::interface::{MetadataUseCase as _, Needs};
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_execution::inbound::use_cases::prepare::interface::DerivedAccess;
use ryo_execution::outbound::digest::interface::DigestPort;
use ryo_execution::outbound::facts::interface::FactsPort;
use ryo_execution::outbound::matching::interface::MatchingPort;
use ryo_execution::outbound::operators::interface::OperatorsPort;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::{AnswerShape, RuleId};
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded, Reuse};
use std::sync::Arc;

/// `lint`, over the ports it calls.
///
/// The lowered tree steps are the only way tree steps run: the removed operator-form
/// switch does not resolve, and the stored rule digest holds no strategy marker.
/// ```compile_fail,E0599
/// use ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl;
/// fn switch(use_case: LintUseCaseImpl) -> LintUseCaseImpl { use_case.unlowered() }
/// ```
pub struct LintUseCaseImpl {
    /// Visits the files on the scheduler's workers and streams their findings.
    visits: FileVisits,
    /// Makes the cross-file steps and renders their findings.
    cross_file: CrossFile,
    /// Lists the files beside what the last run kept, and decides what is used again.
    storing: Storing,
    /// Reads the files the project's lookup consulted again.
    corpus: Arc<dyn CorpusPort>,
    /// Holds the project's rows and finishes the words the cross-file steps read.
    facts: Arc<dyn FactsPort>,
    /// Names the relations, kinds and manifests of the linked languages.
    language: Arc<dyn LanguagePort>,
    /// Decides which files a rule's include and exclude globs govern.
    matching: Arc<dyn MatchingPort>,
    /// Owns rule compilation, independent per-request binding and bounded session plans.
    compilation: Compilation,
    /// Says how much memory the process holds, which bounds the cross-file steps.
    memory: Arc<dyn MemoryPort>,
    /// Pure primitive meanings supplied by the composition edge.
    operators: Arc<dyn OperatorsPort>,
    derived: DerivedAccess,
    /// Takes the findings the rules write.
    rows: Arc<dyn RowSinkPort>,
    /// Digests the rules and files whose results the store keeps between runs.
    digest: Arc<dyn DigestPort>,
    /// Keeps what a run found of each file and rule for the next run, for the build that made it.
    store: Arc<dyn LintStorePort>,
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
    /// `lint` over the ports the edge injects; the visits of the files and the store's listing are built from them here.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "the use case is composed of the ten ports it calls, each injected by the edge on its own: a struct of them would be a second composition between the edge and the use case"
    )]
    pub fn new(
        file_analysis: FileAnalysis,
        cross_file: CrossFile,
        corpus: Arc<dyn CorpusPort>,
        facts: Arc<dyn FactsPort>,
        language: Arc<dyn LanguagePort>,
        matching: Arc<dyn MatchingPort>,
        query_language: Arc<dyn QueryLanguagePort>,
        memory: Arc<dyn MemoryPort>,
        operators: Arc<dyn OperatorsPort>,
        derived: impl Into<DerivedAccess>,
        scheduler: Arc<dyn SchedulerPort>,
        workers: WorkerCount,
        rows: Arc<dyn RowSinkPort>,
        digest: Arc<dyn DigestPort>,
        store: Arc<dyn LintStorePort>,
    ) -> Self {
        let visits = FileVisits::new(
            file_analysis,
            Arc::clone(&facts),
            Arc::clone(&scheduler),
            workers,
            Arc::clone(&rows),
        );
        let storing = Storing::new(
            Arc::clone(&corpus),
            Arc::clone(&digest),
            Arc::clone(&store),
            Arc::clone(&memory),
            scheduler,
            workers,
        );
        Self {
            visits,
            cross_file,
            storing,
            corpus,
            facts,
            language,
            matching,
            compilation: Compilation::new(query_language),
            memory,
            operators,
            derived: derived.into(),
            rows,
            digest,
            store,
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
}

impl LintUseCase for LintUseCaseImpl {
    fn lint(&self, request: LintRequest) -> Result<LintReport, LintError> {
        let analysis = self.visits.analysis();
        analysis.validate_source_language(named_provider(&request.source_language))?;
        let rules: Vec<RuleSpec> = request.rules.iter().map(RuleSpec::from).collect();
        let compiling = tracing::info_span!("compile").entered();
        let compiled = self
            .compilation
            .checked(
                &rules,
                &request.hosted,
                self.digest.as_ref(),
                self.language.as_ref(),
            )?
            .map_err(|refused| LintError::UnknownKind {
                id: rules
                    .get(refused.rule.0)
                    .map(|rule| rule.id.0.to_string())
                    .unwrap_or_default(),
                reason: refused.reason.0.into(),
            })?;
        let plans = compiled.plans();
        let ids = ids_of(&rules);
        let Wanting { placed, wants } = wanting(
            self.matching.as_ref(),
            self.language.as_ref(),
            &rules,
            &plans,
            &ids,
        )?;
        drop(compiling);
        let answering = match request.answer {
            AnswerShape::Findings => Answering::Findings,
            AnswerShape::Rows(limit) => Answering::Rows(rows_answer(&plans, limit)),
        };
        let mut listing = {
            let _listing = tracing::info_span!("list").entered();
            self.storing.listing(&answering).map_err(from_storing)?
        };
        let paths = paths_of(&listing.files);
        let manifests = analysis.read_manifests(&mut listing.files)?;
        let mut project = ProjectHandle {
            symbols: Symbols::default(),
            facts: self.facts.worker()?,
        };
        let first_bucket = first_buckets(&plans);
        let applicability: Vec<_> = rules.iter().map(|rule| rule.applicability).collect();
        let applicability_can_gate = gating_of(&applicability, &plans);
        let needs: Vec<Needs> = plans
            .iter()
            .map(|plan| kernel_metadata().scoped_needs(plan))
            .collect();
        let resolved = crate::application::project_context::service::resolution(
            &manifests,
            &plans,
            &request.session,
            self.digest.as_ref(),
        )
        .map_err(|error| LintError::Corpus {
            reason: error.to_string(),
        })?;
        let stored_rules = self.storing.stored_rules(
            &rules,
            &plans,
            &request.hosted,
            resolved,
            named_provider(&request.source_language),
        );
        let identities =
            self.compilation
                .identify(&compiled, self.digest.as_ref(), stored_rules)?;
        let stored_rules = identities.rules;
        let (nothing_handed, nothing_wanted, nothing_reused) = (
            ReadHanded::default(),
            BucketsWanted::These(Vec::new()),
            Reuse::Whole,
        );
        let unknown = self.language.unknown_reasons();
        let execution = Execution::new(
            Arc::clone(&self.operators),
            self.derived.clone(),
            Arc::clone(&self.digest),
        );
        let digests: Vec<_> = stored_rules.iter().map(|rule| rule.digest).collect();
        let request_failed = |error: &dyn std::fmt::Display| {
            ryo_values::domain::rule_requests::service::failed(
                &RuleId(super::constants::REQUEST.into()),
                error,
            )
        };
        let program = Arc::new(
            execution
                .prepare(PrepareRequest {
                    plans: PlanSource::Compiled(compiled),
                    rules: rules.clone(),
                    digests,
                    hosted: request.hosted.clone(),
                    observations: identities.observations,
                })
                .map_err(|error| request_failed(&error))?,
        );
        let services = execution.begin(&program, &listing.files);
        let metadata = program.metadata();
        let mut setting = RequestVisitSetting {
            program: Arc::clone(&program),
            source_language: named_provider(&request.source_language),
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
                reuse: &nothing_reused,
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
        } = reused(
            self.corpus.as_ref(),
            self.digest.as_ref(),
            &self.visits,
            &setting,
            &listing,
            &manifests,
            &stored_rules,
        )?;
        let ReplayClosure {
            reuse: again,
            buckets: producer_buckets,
        } = program
            .replay_closure(again, &first_bucket)
            .map_err(|error| request_failed(&error))?;
        let reuse_now = again.clone();
        setting.reuse = &reuse_now;
        // A run that makes cross-file steps again reads what the files handed them: the buckets of those rules alone.
        let handed = handed_of(
            listing.read.as_ref(),
            metadata,
            &first_bucket,
            &again,
            &producer_buckets,
        )?;
        setting.handed = &handed.handed;
        setting.wanted_buckets = &handed.wanted;
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
            handed: &handed.handed,
            wanted: &handed.wanted,
        };
        let visit = FileVisit {
            store: listing.read.as_deref(),
            setting,
        };
        let ports = PhasePorts {
            facts: self.facts.as_ref(),
            language: self.language.as_ref(),
            memory: self.memory.as_ref(),
        };
        answer(
            &self.visits,
            &self.cross_file,
            ports,
            self.rows.as_ref(),
            self.store.keeps(),
            self.counting,
            &run,
            &visit,
            &mut project,
        )
    }
}
