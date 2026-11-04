//! Bounded read, parse and per-file evaluation using injected ports.
use crate::application::file_dag::rule_execution::evaluate_rule_with_dag;
// ignore: idiomatic.arch/file-length - this operation owns one bounded worker and the read, parse, admission, evaluation, and retention sequence for its file; its private methods share that invocation state, while domain computations remain separate pure functions.
use super::errors::FileAnalysisError;
use super::types::{FileVisit, Worker};
use crate::domain::admission::errors::AdmissionError;
use crate::domain::admission::service::{admit_elements, admit_view, limits_of};

use crate::domain::budget::service::within;
use crate::domain::coverage::service::partial_of;
use crate::domain::coverage::types::{Recovery, RuleIndex};

use crate::application::file_dag::settings::VisitSetting;
use crate::domain::handing::service::rows_of_file;
use crate::domain::keeping::service::{remap_packed, rows_of};
use crate::domain::keeping::types::Remap;
use crate::domain::placement::service::unplace;
use crate::domain::placement::types::Governing;
use crate::domain::project::types::Projected;
use crate::domain::reading::service::{
    begin, capture_frontier, engine_asked, failure_partials, prepare, prepare_frontier,
    read_failure_demanded, settle as settle_file, start_partials,
};
use crate::domain::reading::types::{EngineAsked, EngineRows, PrimaryFailureKind};
use crate::domain::recording::service::{
    cover, finish_file, frontier_flat, hand_rest_again, interface_bytes, made_ran, moved_words,
    remap_to, replay,
};
use crate::domain::recording::types::Flat;
use crate::domain::render::types::Answering;
use crate::domain::shared_walk::types::Walking;
use crate::domain::table_texts::types::SourceView;
use crate::domain::table_words::service::pack_onto;
use crate::domain::table_words::types::WordSlice;
use crate::domain::workers::types::{Replaying, Tables};
use ryo_relations::domain::relations::types::Words;
use ryo_values::domain::records::types::{
    GoverningRules, HandedNotes, Handing, PlacedRule, Record, RuleFrontier,
};
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::{ArenaBytes, Bucket};
use ryo_values::domain::stores::types::BucketsWanted;
use ryo_values::domain::stores::types::StoredFileNumber;

use crate::domain::rules::errors::RulesError;
use crate::domain::rules::service::governing;

use crate::domain::visit::service::lines_in;
use crate::domain::visit::types::{BorrowedBytes, Keeping};

use crate::domain::finish::types::RuleNumber;
use crate::domain::workers::service::{checking, file_at, govern, governs_rule};

use crate::domain::workers::types::{Checking, FileOrder};
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::corpus::types::PathStat;
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::{DigestBytes, DigestWords};
use crate::outbound::facts::interface::FactsWorker;
use crate::outbound::facts::types::ResidentStability;
use crate::outbound::facts::types::WordsToKeep;
use ryo_relations::domain::relations::collections::relations::filesystem_executable::item::FilesystemExecutableRow;
use ryo_values::domain::files::service::input_path;
use ryo_values::domain::files::types::ExecutablePermission;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::stored_facts::types::{PartialAt, PartialCause, PartialScope, ReadInPart};

use crate::outbound::language::interface::LanguagePort;
use crate::outbound::language::types::{FileReading, FileToRead, ProjectInput};

use crate::outbound::memory::interface::MemoryPort;

use crate::outbound::scheduler::types::JobNumber;
use ryo_language::domain::elements::service::{
    has_incomplete_lexical_stream, has_incomplete_syntax, syntax_acceptance,
};
use ryo_language::domain::elements::types::{Elements, SyntaxAcceptance};
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::symbols::service::clear_symbols;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::collections::relations::filesystem_file::item::FilesystemFileRow;
use ryo_relations::domain::relations::collections::relations::filesystem_line_count::item::FilesystemLineCountRow;
use ryo_relations::domain::relations::types::Rows as _;
use ryo_relations::domain::relations::types::{FileId, Integer, Relation};

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, CorpusInput, FileBytes, FileNumber};
use ryo_values::domain::stores::types::StoredFile;

use ryo_values::domain::records::service::is_as_it_was;
use ryo_values::domain::records::types::{FileOutcome, Freshness, Handed, Ran, Unreadable};

use ryo_values::domain::runs::types::ManifestRefused;
use ryo_values::domain::runs::types::RuleApplicability;

use ryo_values::domain::stored_facts::types::{ArenaBase, Digest, LanguageNumber, ReadFailure};
use std::borrow::Cow;
use std::sync::Arc;
use std::time::Instant;

/// Whether the language read a complete tree or a recovery from missing syntax.
pub(super) fn recovery_of(reading: &FileReading, elements: &Elements) -> Recovery {
    match reading {
        FileReading::Read(_) if has_incomplete_syntax(elements) => {
            if has_incomplete_lexical_stream(elements) {
                Recovery::SyntaxErrors
            } else {
                Recovery::LexicallyCompleteSyntaxErrors
            }
        }
        FileReading::Failed(_) => Recovery::Unparsed,
        FileReading::Read(_) | FileReading::Manifest | FileReading::Unsupported => Recovery::Whole,
    }
}

/// Analysis of one file or the project, with resources supplied by the composition.
pub struct FileAnalysis {
    /// One request's explicitly selected physical execution strategy.
    execution: ryo_values::domain::runs::types::ExecutionOptions,
    /// Reads each file's bytes.
    corpus: Arc<dyn CorpusPort>,
    /// Parses a file into its syntax tree and rows, and names the languages linked.
    language: Arc<dyn LanguagePort>,
    /// Digests the files whose results the store keeps between runs.
    digest: Arc<dyn DigestPort>,
    /// Says how much memory the process holds, which a file's visit keeps within the run's budget.
    memory: Arc<dyn MemoryPort>,
}

impl std::fmt::Debug for FileAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl Worker {
    /// The application creates the worker's bounded request-metadata cache.
    pub(crate) fn new(tables: Tables, facts: Box<dyn FactsWorker>) -> Self {
        Self {
            tables,
            facts,
            old_rows: None,
            activation: Default::default(),
            failure: None,
        }
    }
}

impl FileAnalysis {
    pub(super) fn resource_generation_enabled(tables: &Tables, facts: &dyn FactsWorker) -> bool {
        tables.language.is_some() && facts.resident_stability() == ResidentStability::Stable
    }
    /// Admit one exact linked-provider selection before parsing or retained-data reuse.
    pub(crate) fn validate_source_language(
        &self,
        selected: Option<&ryo_values::domain::descriptions::types::LanguageName>,
    ) -> Result<(), FileAnalysisError> {
        if let Some(selected) = selected
            && self
                .language
                .kind_vocabularies()
                .0
                .iter()
                .filter(|vocabulary| vocabulary.language == *selected)
                .count()
                != 1
        {
            return Err(FileAnalysisError::SourceLanguage {
                name: selected.0.to_string(),
            });
        }
        Ok(())
    }

    /// The parsed relations needed by rules that govern this path and whose applicability is met.
    pub(super) fn active_wanted<'a>(
        tables: &Tables,
        setting: &'a VisitSetting<'_>,
        acceptance: SyntaxAcceptance,
    ) -> Cow<'a, [Relation]> {
        if setting.plans.is_empty() {
            return Cow::Borrowed(setting.wanted);
        }
        // All rules are active for legacy plans, and complete metadata plus accepted syntax
        // activates every policy. Keep the caller's relation slice in both cases.
        if !setting.applicability_can_gate
            || acceptance == SyntaxAcceptance::Accepted
                && setting.applicability.len() == setting.plans.len()
        {
            return Cow::Borrowed(setting.wanted);
        }
        let demanded = crate::domain::file_program::service::observations::demanded_rules(
            &setting.graph,
            |place| governs_rule(tables, RuleIndex(place)) == Governing::Governs,
        );
        Cow::Owned(
            setting
                .wanted
                .iter()
                .copied()
                .filter(|relation| {
                    setting.plans.iter().enumerate().any(|(place, _)| {
                        demanded.get(place).copied().unwrap_or(false)
                            && match setting.applicability.get(place).copied() {
                                Some(RuleApplicability::Always) => true,
                                Some(RuleApplicability::AcceptedSyntax) => {
                                    acceptance == SyntaxAcceptance::Accepted
                                }
                                None => false,
                            }
                            && setting.needs.get(place).is_some_and(|needs| {
                                needs.reads.iter().any(|read| read.relation == *relation)
                            })
                    })
                })
                .collect(),
        )
    }

    /// Whether this setting contains a policy that can change a file's active rules.
    pub(super) fn may_gate_rules(setting: &VisitSetting<'_>, acceptance: SyntaxAcceptance) -> bool {
        setting.applicability_can_gate
            && !(acceptance == SyntaxAcceptance::Accepted
                && setting.applicability.len() == setting.plans.len())
    }

    /// Bind the resources used by every analysis invocation.
    #[must_use]
    pub fn new(
        corpus: Arc<dyn CorpusPort>,
        language: Arc<dyn LanguagePort>,
        digest: Arc<dyn DigestPort>,
        memory: Arc<dyn MemoryPort>,
    ) -> Self {
        Self {
            execution: Default::default(),
            corpus,
            language,
            digest,
            memory,
        }
    }

    /// Bind explicit strategy options at the application composition edge.
    #[must_use]
    pub fn with_execution(
        mut self,
        execution: ryo_values::domain::runs::types::ExecutionOptions,
    ) -> Self {
        self.execution = execution;
        self
    }

    /// The same strategy controls execution and persisted result identity.
    pub(crate) fn execution(&self) -> ryo_values::domain::runs::types::ExecutionOptions {
        self.execution
    }

    /// Pass the strategy to each worker without consulting the environment per file.
    pub(crate) fn kernel_strategy(&self) -> crate::domain::column_kernel::types::KernelStrategy {
        if self.execution.column_kernel {
            crate::domain::column_kernel::types::KernelStrategy::Column
        } else {
            crate::domain::column_kernel::types::KernelStrategy::Existing
        }
    }

    /// Inject the candidate in exact same-binary differential tests.
    #[cfg(test)]
    pub(crate) fn with_column_kernel(mut self, enabled: bool) -> Self {
        self.execution.column_kernel = enabled;
        self
    }
    /// Admits what the language `provider` wrote for the file `number`, against the file's length, its table and its number.
    fn admit(
        &self,
        worker: &Worker,
        wanted: &[Relation],
        number: FileId,
        provider: LanguageNumber,
    ) -> Result<(), FileAnalysisError> {
        let refused = |code: AdmissionError| FileAnalysisError::Provider {
            provider: self
                .language
                .linked()
                .0
                .get(provider.0)
                .map(|name| name.0.to_string())
                .unwrap_or_default(),
            code,
        };
        let tables = &worker.tables;
        let limits = limits_of(&tables.elements, &tables.bytes, number);
        admit_elements(&tables.elements, &limits).map_err(refused)?;
        for relation in wanted {
            admit_view(&worker.facts.rows(*relation)?, &limits).map_err(refused)?;
        }
        Ok(())
    }

    /// Removes rules whose declared syntax applicability is not met and records why each one was
    /// skipped for this file. The skip is separate from relation partiality: it says the rule was
    /// not applicable, not that its input was read incompletely.
    pub(super) fn gate_rules(
        tables: &mut Tables,
        setting: &VisitSetting<'_>,
        file: FileId,
        acceptance: SyntaxAcceptance,
    ) -> Vec<bool> {
        let demanded = crate::domain::file_program::service::observations::demanded_rules(
            &setting.graph,
            |place| governs_rule(tables, RuleIndex(place)) == Governing::Governs,
        );
        let mut skipped = vec![false; setting.plans.len()];
        for (place, blocked) in skipped.iter_mut().enumerate() {
            if !demanded.get(place).copied().unwrap_or(false) {
                continue;
            }
            let applicability = setting.applicability.get(place).copied();
            *blocked = match applicability {
                Some(RuleApplicability::Always) => false,
                Some(RuleApplicability::AcceptedSyntax) => acceptance != SyntaxAcceptance::Accepted,
                None => true,
            };
            if !*blocked {
                continue;
            }
            unplace(
                &mut tables.mask,
                RuleNumber(u32::try_from(place).unwrap_or(u32::MAX)),
            );
            let reason = match (applicability, acceptance) {
                (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Rejected) => {
                    let (Some(rule), Some(path)) = (
                        setting.ids.get(place),
                        setting
                            .paths
                            .get(usize::try_from(file.0).unwrap_or(usize::MAX)),
                    ) else {
                        continue;
                    };
                    tables
                        .skipped
                        .push(ryo_values::domain::runs::types::Skipped {
                            rule: rule.clone(),
                            path: path.clone(),
                            reason: ryo_values::domain::runs::types::SkipReason::SyntaxRejected,
                        });
                    continue;
                }
                (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Undetermined) => {
                    "rule skipped: syntax acceptance undetermined"
                }
                (Some(RuleApplicability::AcceptedSyntax), SyntaxAcceptance::Accepted)
                | (Some(RuleApplicability::Always), _) => continue,
                (None, _) => "rule skipped: applicability metadata unavailable",
            };
            let Some(rule) = setting.ids.get(place) else {
                continue;
            };
            let Some(path) = setting
                .paths
                .get(usize::try_from(file.0).unwrap_or(usize::MAX))
            else {
                continue;
            };
            tables
                .unknowns
                .push(crate::domain::coverage::types::Unknown {
                    rule: rule.clone(),
                    path: path.clone(),
                    message: crate::domain::coverage::types::Message(reason.into()),
                });
        }
        skipped
    }

    /// The rest of a visit of a file the language read: the per-file steps of the rules that govern it run on its rows, the words of
    /// what leaves it wait in the buckets, and the rows are released.
    fn read_file(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        file: FileId,
        semantic: &super::semantic::SemanticState,
    ) -> Result<(), FileAnalysisError> {
        let _route_counts =
            crate::application::file_dag::route_counts::FileScope::begin(file.0 as usize);
        #[cfg(feature = "diagnostic-row-progress")]
        if let Some(path) = visit.setting.paths.get(file.0 as usize) {
            crate::application::file_dag::route_counts::file_name(path.0.as_ref());
        }
        let (tables, facts, activation) = (
            &mut worker.tables,
            &mut worker.facts,
            &mut worker.activation,
        );
        crate::application::file_dag::derived::selection::prepare_request(
            &mut tables.pool,
            &visit.setting.operators,
        );
        let graph = &visit.setting.graph;
        // Preserve path scope, but keep syntax-gated rules out of the shared element walk when this provider did not accept the file.
        let acceptance = syntax_acceptance(&tables.elements);
        let scoped_mask =
            Self::may_gate_rules(&visit.setting, acceptance).then(|| tables.mask.clone());
        let skipped = scoped_mask
            .as_ref()
            .map(|_| Self::gate_rules(tables, &visit.setting, file, acceptance));
        let active_wanted = Self::active_wanted(tables, &visit.setting, acceptance);
        // Keep physical reader partiality before any rule's Rows output can relax evaluation coverage.
        let physical_partials = tables.partials.clone();
        // Stored indexes keep their existing rectangular admission. The graph's
        // completeness proof also considers the actual native tree reader below.
        let mut selectable = Vec::new();
        let mut complete = Vec::new();
        for relation in active_wanted.iter() {
            let view = facts.rows(*relation)?;
            if crate::domain::table_model::service::rectangular(view)
                && view.kinds == ryo_relations::domain::relations::service::kinds(*relation)
                && view
                    .columns
                    .first()
                    .is_none_or(|column| column.0.len() <= u32::MAX as usize)
            {
                selectable.push(*relation);
                if crate::domain::table_cells::service::native_form(*relation).is_none()
                    && crate::domain::table_model::service::complete_for_reuse(
                        *relation,
                        &crate::domain::table_model::types::Rows::Stored(view),
                        &physical_partials,
                        facts.resident_stability() == ResidentStability::Stable,
                    )
                {
                    complete.push(*relation);
                }
            }
        }
        if tables.language.is_some() && facts.resident_stability() == ResidentStability::Stable {
            for &(relation, form) in crate::domain::table_cells::service::native_relations() {
                let admitted = crate::domain::table_model::service::complete_for_reuse(
                    relation,
                    &crate::domain::table_model::types::Rows::Tree(
                        crate::domain::table_cells::types::TreeRows {
                            elements: &tables.elements,
                            file,
                            form,
                        },
                    ),
                    &physical_partials,
                    true,
                );
                tracing::trace!(target: "ryo::diagnostic::file_program", event = "native_relation_admission",
                    file = file.0, relation = ?relation, admitted,
                    acceptance = ?acceptance, incomplete_syntax = has_incomplete_syntax(&tables.elements),
                    physically_partial = physical_partials.iter().any(|partial| partial.relation == relation));
                if admitted && !complete.contains(&relation) {
                    complete.push(relation);
                }
            }
            tracing::trace!(target: "ryo::diagnostic::file_program", event = "complete_relations",
                complete = ?complete);
        }
        // The generation owner is unconditional. The benchmark switch changes
        // retention only; every class keeps the same executor and semantics.
        let stable = facts.resident_stability() == ResidentStability::Stable;
        let scope = crate::application::file_dag::scope::ExecutionScope::new(
            std::sync::Arc::clone(graph),
            crate::application::file_dag::scope::ScopePhase::File(file),
            stable,
            visit.setting.executor,
        )?;
        let owner = std::rc::Rc::clone(scope.owner());
        let executor = scope.executor();
        let demanded =
            crate::domain::file_program::service::observations::demanded_rules(graph, |place| {
                governs_rule(tables, RuleIndex(place)) == Governing::Governs
            });
        let file_dag = {
            let schedule = activation.activate(
                graph,
                |index| {
                    demanded.get(index).copied().unwrap_or(false)
                        && !skipped
                            .as_ref()
                            .and_then(|skipped| skipped.get(index))
                            .copied()
                            .unwrap_or(false)
                },
                &complete,
            );
            let mut execution =
                crate::application::file_dag::types::FileDagExecution::new_for_schedule(
                    &schedule,
                    std::rc::Rc::clone(&owner),
                    scope.completion(
                        RuleIndex(0),
                        crate::application::file_dag::scope::ScopePartials::File {
                            evaluation: if matches!(
                                visit.setting.answering,
                                crate::domain::render::types::Answering::Rows(_)
                            ) {
                                &[]
                            } else {
                                &physical_partials
                            },
                            publication: &physical_partials,
                        },
                    )?,
                );
            if self.execution.node_result_reuse
                == ryo_values::domain::runs::types::NodeResultReuse::Disabled
            {
                execution.limit = 0;
            }
            std::cell::RefCell::new(execution)
        };
        file_dag.borrow().prepare_generation_resources(
            Self::resource_generation_enabled(tables, facts.as_ref()),
            active_wanted.as_ref(),
            &physical_partials,
            &selectable,
            visit.setting.plans,
            &tables.pool,
        );

        // The steps of the rules that select from the elements are decided in one pass over them.
        let execution_diagnostics =
            tracing::enabled!(target: "ryo::diagnostic::execution", tracing::Level::TRACE);
        let shared_started = execution_diagnostics.then(Instant::now);
        let mut classes = crate::domain::shared_walk::types::SharedClasses {
            matchers: Some(Arc::clone(&tables.pool.matchers)),
            kind_vocabulary: crate::domain::kind_sets::service::prepare_vocabulary(
                &mut tables.pool.kind_vocabulary,
                &tables.elements,
                &tables.symbols,
                SourceView(&tables.bytes.0),
            ),
            ..crate::domain::shared_walk::types::SharedClasses::default()
        };
        let classes = {
            let mut native_scratch =
                crate::domain::table_scratch::types::OperatorScratch::default();
            let Tables {
                mask,
                elements,
                symbols,
                bytes,
                shared,
                pool,
                ..
            } = &mut *tables;
            crate::application::file_dag::types::FileDagExecution::prepare_native(
                &file_dag,
                shared,
                Walking {
                    plans: visit.setting.plans,
                    bindings: visit.setting.bindings,
                    paths: visit.setting.paths,
                    mask,
                    file,
                    elements,
                    symbols,
                    source: SourceView(&bytes.0),
                    scratch: &mut native_scratch,
                    room: &mut classes,
                    steps: pool.unlowered,
                },
                pool.kernel_strategy,
                pool.counting,
                executor,
            )?
        };
        if let Some(scoped_mask) = scoped_mask {
            tables.mask = scoped_mask;
        }
        if let Some(started) = shared_started {
            tracing::trace!(target: "ryo::diagnostic::execution", phase = "native_shared_walk", file = file.0, elapsed_ns = started.elapsed().as_nanos() as u64);
        }
        let rules_started = execution_diagnostics.then(Instant::now);
        let evaluated = Self::evaluate_file_rules(
            tables,
            facts.as_mut(),
            visit,
            file,
            skipped.as_deref(),
            &file_dag,
            &executor,
            &classes,
            semantic,
        );
        {
            let dag = &file_dag;
            tables.current_observations = dag.borrow().observation_snapshot();
            if let Some(root) = &tables.current_observations {
                tables.observations.push(
                    crate::domain::coverage::types::FileObservationReadiness {
                        file,
                        root: std::sync::Arc::clone(root),
                    },
                );
            }
        }
        if let Some(started) = rules_started {
            tracing::trace!(target: "ryo::diagnostic::execution", phase = "rules_and_frontiers", file = file.0, elapsed_ns = started.elapsed().as_nanos() as u64, success = evaluated.is_ok());
        }
        {
            let dag = &file_dag;
            let dag = dag.borrow();
            tracing::trace!(target: "ryo::diagnostic::file_program", event = "dag_file",
                file = file.0, demands = dag.demands, hits = dag.hits,
                executions = dag.executions.iter().sum::<u64>(),
                releases = dag.releases, retained_bytes = dag.retained_bytes,
                peak_bytes = dag.peak_bytes, unretained = dag.unretained,
                required_bytes=dag.required_bytes, peak_required_bytes=dag.peak_required_bytes,
                overlap_bytes=dag.overlap_bytes, peak_owned_bytes=dag.peak_owned_bytes,
                required_releases=dag.required_releases,
                required_metadata_bytes=dag.required_metadata_bytes());
        }
        {
            let dag = &file_dag;
            dag.borrow().report_resources(file);
        }

        // No index handle or allocation survives release or the next visit, even for the same FileId.
        let settle_started = execution_diagnostics.then(Instant::now);
        evaluated?;
        settle_file(tables, visit.setting.answering);
        {
            let dag = &file_dag;
            dag.borrow_mut().release_population();
        }
        facts.release()?;
        if let Some(started) = settle_started {
            tracing::trace!(target: "ryo::diagnostic::execution", phase = "settle_and_release", file = file.0, elapsed_ns = started.elapsed().as_nanos() as u64);
        }
        Ok(())
    }

    /// Evaluate active rules against the same immutable physical file generation.
    fn evaluate_file_rules(
        tables: &mut Tables,
        facts: &mut dyn FactsWorker,
        visit: &FileVisit<'_>,
        file: FileId,
        skipped: Option<&[bool]>,

        file_dag: &std::cell::RefCell<crate::application::file_dag::types::FileDagExecution>,
        executor: &crate::application::file_dag::executor::ScopedExecutor<'_>,
        classes: &crate::domain::shared_walk::types::SharedClasses,
        semantic: &super::semantic::SemanticState,
    ) -> Result<(), FileAnalysisError> {
        let demanded = crate::domain::file_program::service::observations::demanded_rules(
            &visit.setting.graph,
            |place| governs_rule(tables, RuleIndex(place)) == Governing::Governs,
        );
        for rule in crate::domain::file_program::service::observations::rule_order(
            visit.setting.observations,
            visit.setting.plans.len(),
        ) {
            if !demanded.get(rule.0).copied().unwrap_or(false) {
                {
                    let dag = file_dag;
                    dag.borrow_mut().skip_observation_rule(rule.0);
                }
                continue;
            }
            if skipped
                .and_then(|skipped| skipped.get(rule.0))
                .copied()
                .unwrap_or(false)
            {
                {
                    let dag = file_dag;
                    dag.borrow_mut().skip_observation_rule(rule.0);
                }
                continue;
            }
            // The worker already owns these columns. This callback only borrows resident views;
            // source reads and retained-word writes stay in this application operation.
            let rows = &*facts;
            // A view the store refuses stops the file: the step that asked for it is not read as given no rows.
            let refused = std::cell::RefCell::new(None);
            let diagnostic = tracing::enabled!(
                target: "ryo::diagnostic::rule_file",
                tracing::Level::TRACE
            );
            let identity = diagnostic.then(|| {
                let rule_id = visit
                    .setting
                    .ids
                    .get(rule.0)
                    .map_or("<unknown>", |id| id.0.as_ref());
                let file_path = usize::try_from(file.0)
                    .ok()
                    .and_then(|index| visit.setting.paths.get(index))
                    .map_or("<unknown>", |path| path.0.as_ref());
                (rule_id, file_path)
            });
            let started = diagnostic.then(Instant::now);
            if let Some((rule_id, file_path)) = identity {
                tracing::trace!(
                    target: "ryo::diagnostic::rule_file",
                    event = "rule_file_start",
                    rule = rule_id,
                    file = file_path,
                );
            }
            let evaluated = evaluate_rule_with_dag(
                tables,
                &visit.setting,
                rule,
                file,
                &|relation| match rows.rows(relation) {
                    Ok(view) => Some(view),
                    Err(error) => {
                        *refused.borrow_mut() = Some(error);
                        None
                    }
                },
                classes,
                file_dag,
                executor,
                Some(semantic),
            );
            let outcome = match refused.take() {
                Some(error) => Err(FileAnalysisError::from(error)),
                None => evaluated.map_err(FileAnalysisError::from),
            };
            if let (Some(started), Some((rule_id, file_path))) = (started, identity) {
                tracing::trace!(
                    target: "ryo::diagnostic::rule_file",
                    event = "rule_file_end",
                    rule = rule_id,
                    file = file_path,
                    elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
                    outcome = if outcome.is_ok() { "ok" } else { "error" },
                );
            }
            let mut frontier = match outcome {
                Ok(frontier) => frontier,
                Err(FileAnalysisError::Rules(RulesError::ObservationIncomplete { id, reason })) => {
                    let path = visit
                        .setting
                        .paths
                        .get(usize::try_from(file.0).unwrap_or(usize::MAX))
                        .ok_or(FileAnalysisError::Rules(
                            RulesError::ObservationIncomplete {
                                id: id.clone(),
                                reason: reason.clone(),
                            },
                        ))?;
                    tables
                        .unknowns
                        .push(crate::domain::coverage::types::Unknown {
                            rule: id,
                            path: path.clone(),
                            message: crate::domain::coverage::types::Message(reason.into()),
                        });
                    {
                        let dag = file_dag;
                        dag.borrow_mut().finish_rule(rule.0);
                    }
                    continue;
                }
                Err(FileAnalysisError::Rules(RulesError::SemanticIncomplete { id, reason })) => {
                    let path = visit
                        .setting
                        .paths
                        .get(usize::try_from(file.0).unwrap_or(usize::MAX))
                        .ok_or(FileAnalysisError::Rules(RulesError::SemanticIncomplete {
                            id: id.clone(),
                            reason: reason.clone(),
                        }))?;
                    tables
                        .unknowns
                        .push(crate::domain::coverage::types::Unknown {
                            rule: id,
                            path: path.clone(),
                            message: crate::domain::coverage::types::Message(reason.into()),
                        });
                    semantic.mark_incomplete();
                    {
                        let dag = file_dag;
                        dag.borrow_mut().finish_rule(rule.0);
                    }
                    continue;
                }
                Err(error) => return Err(error),
            };
            let inputs = visit
                .setting
                .plans
                .get(rule.0)
                .map_or(0, |plan| plan.frontier.len());
            for place in 0..inputs {
                if governs_rule(tables, rule) == Governing::DoesNotGovern
                    && !visit
                        .setting
                        .graph
                        .observations
                        .as_ref()
                        .and_then(|observations| observations.observation_frontiers.get(rule.0))
                        .and_then(|frontiers| frontiers.get(place))
                        .copied()
                        .unwrap_or(false)
                {
                    continue;
                }
                let Some(bucket) = prepare_frontier(
                    tables,
                    &visit.setting,
                    rule,
                    file,
                    ryo_values::domain::plan_parts::types::InputNumber(place),
                    &mut frontier,
                ) else {
                    continue;
                };
                facts.keep(bucket, WordsToKeep(&tables.packed))?;
                file_dag.borrow_mut().commit_frontier(
                    rule.0,
                    ryo_values::domain::plan_parts::types::InputNumber(place),
                );
                // An observation consumer may demand an excluded producer's
                // frontier. Keep those words for the consumer, but only a
                // governing rule contributes findings to the answer.
                if visit.setting.keeping == Keeping::Keeps
                    && governs_rule(tables, rule) == Governing::Governs
                {
                    capture_frontier(tables, visit.setting.answering, bucket);
                }
            }
            {
                let dag = file_dag;
                dag.borrow_mut().finish_rule(rule.0);
            }
        }
        Ok(())
    }

    /// Reads primary inventoried manifests once before project preparation, retaining genuine read failures.
    /// Consulted project dependencies use their separate lookup route and never become primary failure events.
    pub(crate) fn read_manifests(
        &self,
        files: &mut [CorpusInput],
    ) -> Result<
        Vec<ryo_language::inbound::use_cases::project::types::ManifestFile>,
        FileAnalysisError,
    > {
        use crate::outbound::language::types::ManifestReading;
        use ryo_language::inbound::use_cases::project::types::{ManifestFile, ManifestText};
        use ryo_values::domain::files::types::InputReadFailure;
        let mut manifests = Vec::new();
        let mut text = FileBytes::default();
        for input in files {
            if self.language.reads_manifest(input_path(input)) != ManifestReading::Read {
                continue;
            }
            let CorpusInput::Available(file) = input else {
                continue;
            };
            text.0.clear();
            match self.corpus.read(file, &mut text) {
                Ok(()) => manifests.push(ManifestFile {
                    path: file.path.clone(),
                    text: ManifestText(String::from_utf8_lossy(&text.0).into()),
                }),
                Err(CorpusError::Unreadable { reason, .. }) => {
                    text.0.clear();
                    *input = CorpusInput::ReadFailed {
                        entry: file.clone(),
                        reason: InputReadFailure(reason.into()),
                    };
                }
                Err(error) => {
                    text.0.clear();
                    return Err(error.into());
                }
            }
        }
        Ok(manifests)
    }

    /// The visit of one file, in the worker that took its job: read it, decide which rules govern it, parse it, and run their
    /// per-file steps on its rows. What it could not read is noted in the worker; what stops the run is its failure.
    pub(crate) fn visit_file(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        files: &[CorpusInput],
        job: JobNumber,
    ) {
        if let Err(error) = self.visit(worker, visit, files, job) {
            // The first failure is the one the run answers with.
            if worker.failure.is_none() {
                worker.failure = Some(error);
            }
        }
    }

    /// The visit of one file, answering what stops the run.
    fn visit(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        files: &[CorpusInput],
        job: JobNumber,
    ) -> Result<(), FileAnalysisError> {
        let setting = &visit.setting;
        let at = FileOrder(usize::try_from(job.0).unwrap_or(usize::MAX));
        let Some(file) = file_at(files, at) else {
            return Ok(());
        };
        let number = FileId(job.0);
        if govern(setting, &mut worker.tables, input_path(file)) == Governing::DoesNotGovern {
            return Ok(());
        }
        if checking(at) == Checking::Now {
            let mask = &worker.tables.mask;
            within(self.memory.resident_bytes()?, setting.budget, || {
                governing(mask, setting.ids)
            })?;
        }
        let file = match file {
            CorpusInput::Available(entry) => entry,
            CorpusInput::ReadFailed { entry, reason } => {
                return self.read_failed(
                    worker,
                    visit,
                    number,
                    &entry.path,
                    entry.executable,
                    &reason.0,
                    PrimaryFailureKind::Disk,
                    true,
                );
            }
            CorpusInput::Unreadable { path, reason } => {
                return self.read_failed(
                    worker,
                    visit,
                    number,
                    path,
                    ExecutablePermission::Unknown,
                    &reason.0,
                    PrimaryFailureKind::Disk,
                    false,
                );
            }
        };
        let kept = setting.records.get(at.0).copied().flatten();
        // A file the system says is as it was is not read: what came of it is used again.
        if let Some(record) = kept.filter(|record| {
            !read_failure_demanded(
                setting.wanted,
                &worker.tables.mask,
                setting.needs,
                setting.ids.len(),
            ) && is_as_it_was(record, file) == Freshness::AsItWas
        }) {
            replay(&mut worker.tables, number, at, record, file, record.digest);
            return Self::hand_again(worker, visit, at, number, record);
        }
        worker.tables.bytes.0.clear();
        if let Err(error) = self.corpus.read(file, &mut worker.tables.bytes) {
            worker.tables.bytes.0.clear();
            let kind = if matches!(error, CorpusError::Unreadable { .. }) {
                PrimaryFailureKind::Disk
            } else {
                PrimaryFailureKind::Other
            };
            return self.read_failed(
                worker,
                visit,
                number,
                &file.path,
                file.executable,
                &error.to_string(),
                kind,
                true,
            );
        }
        // Only a store reads the digest of a file's bytes.
        let digest =
            if *setting.answering == Answering::Findings && setting.keeping == Keeping::Keeps {
                self.digest.digest(DigestBytes(&worker.tables.bytes.0))
            } else {
                Digest([0; 16])
            };
        // A file written again with the same bytes is as it was too.
        if let Some(record) =
            kept.filter(|record| record.digest == digest && record.executable == file.executable)
        {
            replay(&mut worker.tables, number, at, record, file, digest);
            return Self::hand_again(worker, visit, at, number, record);
        }
        let outcome = self.parse_file(worker, visit, file, number)?;
        let outcome = Self::stage_outcome(visit.store, outcome)?;
        let acceptance = syntax_acceptance(&worker.tables.elements);
        cover(&mut worker.tables, setting, number, &file.path, acceptance);
        finish_file(
            &mut worker.tables,
            setting,
            at,
            file,
            digest,
            FileNumber(number.0),
            outcome,
        );
        Ok(())
    }

    /// Executes demanded engine facts for a primary input failure without constructing source or syntax.
    fn read_failed(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        number: FileId,
        path: &FilePath,
        executable: ExecutablePermission,
        detail: &str,
        kind: PrimaryFailureKind,
        observed_file: bool,
    ) -> Result<(), FileAnalysisError> {
        use ryo_language::domain::symbols::service::intern_derived;
        use ryo_language::domain::symbols::types::ByteSlice;
        use ryo_relations::domain::relations::collections::relations::filesystem_read_failure::item::FilesystemReadFailureRow;
        use ryo_relations::domain::relations::types::{Offset, Span};
        use ryo_relations::domain::terms::types::NameText;

        let setting = &visit.setting;
        worker.tables.bytes.0.clear();
        prepare(&mut worker.tables);
        clear_symbols(&mut worker.tables.symbols);
        worker.tables.language = None;
        worker.facts.release()?;
        let asked = engine_asked(setting.wanted);
        let mut said = failure_partials(setting.wanted, number, detail, observed_file);
        Self::write_engine_rows(
            worker,
            EngineAsked {
                file: if observed_file {
                    asked.file
                } else {
                    EngineRows::NotAsked
                },
                lines: EngineRows::NotAsked,
                ..asked
            },
            number,
            executable,
            &mut said,
        )?;
        if asked.read_failure == EngineRows::Asked && kind == PrimaryFailureKind::Disk {
            let detail =
                intern_derived(&mut worker.tables.symbols, ByteSlice(&[]), NameText(detail));
            worker.facts.columns_mut()?.write(FilesystemReadFailureRow {
                path: number,
                detail,
                span: Span {
                    start: Offset(0),
                    end: Offset(0),
                },
            });
        }
        start_partials(&mut worker.tables, &said, Recovery::Whole, setting.wanted);
        worker.tables.unread.push(Unread {
            path: path.clone(),
            reason: Unreadable::Failed(ReadFailure(detail.into())),
        });
        let acceptance = syntax_acceptance(&worker.tables.elements);
        if asked.any == EngineRows::Asked {
            index_kinds(&mut worker.tables.elements);
            let semantic =
                super::semantic::SemanticState::new(number, &worker.tables.elements, None);
            self.read_file(worker, visit, number, &semantic)?;
        } else if Self::may_gate_rules(setting, acceptance) {
            drop(Self::gate_rules(
                &mut worker.tables,
                setting,
                number,
                acceptance,
            ));
        }
        cover(&mut worker.tables, setting, number, path, acceptance);
        // Failure facts are invocation-local: retry the real input next time even if metadata is unchanged.
        if matches!(setting.answering, Answering::Findings) {
            worker.tables.unrecorded.push(path.clone());
        }
        Ok(())
    }

    /// Only successfully validated and digested fresh outcomes cross this IO boundary.
    pub(super) fn stage_outcome(
        read: Option<&dyn crate::outbound::lint_store::interface::ReadStore>,
        mut outcome: Option<FileOutcome>,
    ) -> Result<Option<FileOutcome>, FileAnalysisError> {
        let Some(read) = read else {
            return Ok(outcome);
        };
        let ran = match outcome.as_mut() {
            Some(FileOutcome::Read(ran) | FileOutcome::Unread { ran: Some(ran), .. }) => Some(ran),
            None | Some(FileOutcome::Unread { ran: None, .. }) => None,
        };
        if let Some(ran) = ran
            && let Handing::Made(handed) = &mut ran.handed
        {
            for bucket in &mut handed.buckets {
                let words = std::mem::replace(
                    &mut bucket.words,
                    ryo_values::domain::records::types::RecordWords::Inline(PackedWords::default()),
                );
                bucket.words = match words {
                    ryo_values::domain::records::types::RecordWords::Inline(words) => {
                        read.stage_words(words)?
                    }
                    staged @ ryo_values::domain::records::types::RecordWords::Staged(_) => staged,
                };
            }
        }
        Ok(outcome)
    }

    /// Hands what the store kept of a file the run uses again to the steps that read across files, when they are made again.
    pub(super) fn hand_again(
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        at: FileOrder,
        number: FileId,
        record: &Record,
    ) -> Result<(), FileAnalysisError> {
        let setting = &visit.setting;
        if setting.replaying != Replaying::FindingsAndHanded {
            return Ok(());
        }
        let ran = match &record.outcome {
            FileOutcome::Read(ran) | FileOutcome::Unread { ran: Some(ran), .. } => ran,
            FileOutcome::Unread { ran: None, .. } => return Ok(()),
        };
        let map = remap_to(&worker.tables, number);
        let mut out = Words::default();
        match &ran.handed {
            Handing::Made(handed) => {
                let notes = match &handed.notes {
                    HandedNotes::Noted(notes) => Some(notes),
                    HandedNotes::Nothing => None,
                };
                hand_rest_again(
                    &mut worker.tables,
                    at,
                    number,
                    &handed.text,
                    notes,
                    &handed.governing,
                    &handed.partials,
                    handed.observations.as_ref(),
                );
                let mut loaded = PackedWords::default();
                for each in &handed.buckets {
                    // Filtering must precede any staged read, including an invalid unrequested descriptor.
                    if let BucketsWanted::These(buckets) = setting.wanted_buckets
                        && buckets.binary_search(&each.bucket).is_err()
                    {
                        continue;
                    }
                    let words = match &each.words {
                        ryo_values::domain::records::types::RecordWords::Inline(words) => words,
                        ryo_values::domain::records::types::RecordWords::Staged(span) => {
                            let read = visit.store.ok_or(
                                crate::outbound::lint_store::errors::LintStoreError::NothingRead,
                            )?;
                            read.read_staged(span, &mut loaded)?;
                            &loaded
                        }
                    };
                    Self::hand_words_again(worker, setting, each.bucket, words, &map, &mut out)?;
                }
            }
            // What the store kept of the file is handed as it is read, its words moved as they are unpacked: none is copied first.
            Handing::Kept => {
                let Ok(place) = setting
                    .stored
                    .files
                    .binary_search_by(|each| each.path.cmp(&record.path))
                else {
                    return Ok(());
                };
                let file = StoredFileNumber(u32::try_from(place).unwrap_or(u32::MAX));
                let kept = setting.handed;
                let covered = rows_of_file(&kept.covered, file, |each| each.file).first();
                hand_rest_again(
                    &mut worker.tables,
                    at,
                    number,
                    rows_of_file(&kept.texts, file, |each| each.file)
                        .first()
                        .map_or(&ArenaBytes::default(), |each| &each.text),
                    rows_of_file(&kept.notes, file, |each| each.file)
                        .first()
                        .map(|each| &each.notes),
                    covered.map_or(&GoverningRules::default(), |each| &each.governing),
                    covered.map_or(&[][..], |each| &each.partials),
                    match rows_of_file(&kept.observations, file, |each| each.file) {
                        [one] => Some(&one.root),
                        _ => None,
                    },
                );
                let mut loaded = PackedWords::default();
                for each in rows_of_file(&kept.frontier, file, |each| each.file) {
                    if let BucketsWanted::These(buckets) = setting.wanted_buckets
                        && buckets.binary_search(&each.bucket).is_err()
                    {
                        continue;
                    }
                    let words = match &each.words {
                        ryo_values::domain::stores::types::ReadWordPayload::Inline(words) => words,
                        ryo_values::domain::stores::types::ReadWordPayload::Old(span) => {
                            let read = visit.store.ok_or(
                                crate::outbound::lint_store::errors::LintStoreError::NothingRead,
                            )?;
                            if worker.old_rows.is_none() {
                                worker.old_rows = read.begin_old_rows(setting.wanted_buckets)?;
                            }
                            if let Some(batch) = worker.old_rows.as_mut() {
                                batch.read(span, &mut loaded)?;
                            } else {
                                read.read_old(span, &mut loaded)?;
                            }
                            &loaded
                        }
                    };
                    Self::hand_words_again(worker, setting, each.bucket, words, &map, &mut out)?;
                }
            }
        }
        Ok(())
    }

    /// Hands the words a file handed the last run in `bucket`, moved as `map` says, to the worker's bucket, when the run makes the
    /// steps that read the bucket again; words that cannot be moved are left out. `out` is room the words are moved into.
    fn hand_words_again(
        worker: &mut Worker,
        setting: &VisitSetting<'_>,
        bucket: Bucket,
        words: &PackedWords,
        map: &Remap,
        out: &mut Words,
    ) -> Result<(), FileAnalysisError> {
        if let BucketsWanted::These(buckets) = setting.wanted_buckets
            && buckets.binary_search(&bucket).is_err()
        {
            return Ok(());
        }
        out.0.clear();
        if remap_packed(words, map, out).is_some() {
            let packed = &mut worker.tables.packed;
            packed.0.clear();
            pack_onto(WordSlice(&out.0), packed);
            worker
                .facts
                .keep(bucket, WordsToKeep(&worker.tables.packed))?;
        }
        Ok(())
    }

    /// The read of a file the run has not kept: the language parses it, what it wrote is admitted, and the per-file steps run on
    /// its rows. Answers what the store keeps of it.
    fn parse_file(
        &self,
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        file: &CorpusEntry,
        number: FileId,
    ) -> Result<Option<FileOutcome>, FileAnalysisError> {
        let setting = &visit.setting;
        let base = begin(&mut worker.tables);
        let source = FileToRead {
            source_language: setting.source_language,
            path: &file.path,
            file: number,
            bytes: &worker.tables.bytes,
            wanted: setting.wanted,
            notes: setting.noted.into(),
            roles: setting.roled.into(),
        };
        let mut writer = worker.facts.columns_mut()?;
        let parse_started =
            tracing::enabled!(target: "ryo::diagnostic::execution", tracing::Level::TRACE)
                .then(Instant::now);
        let reading = self.language.parse(
            source,
            &mut worker.tables.elements,
            &mut worker.tables.symbols,
            &mut writer,
        );
        if let Some(started) = parse_started {
            tracing::trace!(target: "ryo::diagnostic::execution", phase = "provider_parse", file = number.0, elapsed_ns = started.elapsed().as_nanos() as u64);
        }
        let mut said: Vec<ReadInPart> = std::mem::take(&mut writer.partials);
        let acceptance = syntax_acceptance(&worker.tables.elements);
        let active_wanted = Self::active_wanted(&worker.tables, setting, acceptance);
        // The language that read the file is kept with what it noted of it.
        worker.tables.language = match &reading {
            FileReading::Read(language) => Some(*language),
            FileReading::Manifest | FileReading::Unsupported | FileReading::Failed(_) => None,
        };
        // Rows a file left in the worker's tables before it was found unreadable are released here.
        if !matches!(reading, FileReading::Read(_)) {
            worker.facts.release()?;
        }
        // What the language wrote is admitted before the store keeps anything or a rule reads it.
        if let FileReading::Read(provider) = &reading {
            self.admit(worker, active_wanted.as_ref(), number, *provider)?;
        }
        let asked = engine_asked(active_wanted.as_ref());
        Self::write_engine_rows(worker, asked, number, file.executable, &mut said)?;
        // The relations this file is read in part: those a reader said so of, and all of code if what was read is a recovery.
        let everything = recovery_of(&reading, &worker.tables.elements);
        start_partials(
            &mut worker.tables,
            &said,
            everything,
            active_wanted.as_ref(),
        );
        let semantic_needed = setting.plans.iter().any(|plan| {
            plan.steps.iter().any(|step| {
                matches!(
                    step.operation,
                    ryo_values::domain::plans::types::Operation::RenderSyntax { .. }
                )
            })
        });
        let session = if semantic_needed {
            worker.tables.language.and_then(|language| {
                let source = FileToRead {
                    source_language: setting.source_language,
                    path: &file.path,
                    file: number,
                    bytes: &worker.tables.bytes,
                    wanted: setting.wanted,
                    notes: setting.noted.into(),
                    roles: setting.roled.into(),
                };
                self.language
                    .begin_semantics(language, source, &worker.tables.elements)
            })
        } else {
            None
        };
        let semantic =
            super::semantic::SemanticState::new(number, &worker.tables.elements, session);
        Ok(match reading {
            FileReading::Read(_) | FileReading::Manifest => {
                index_kinds(&mut worker.tables.elements);
                self.read_file(worker, visit, number, &semantic)?;
                if semantic.is_incomplete() {
                    None
                } else {
                    self.recorded(worker, visit, number, base)
                        .map(FileOutcome::Read)
                }
            }
            unread @ (FileReading::Failed(_) | FileReading::Unsupported) => {
                // What no language read has no tree and no names; the rules that ask for the file still run.
                let reason = match unread {
                    FileReading::Failed(reason) => Unreadable::Failed(reason),
                    FileReading::Read(_) | FileReading::Manifest | FileReading::Unsupported => {
                        Unreadable::NoLanguage
                    }
                };
                worker.tables.unread.push(Unread {
                    path: file.path.clone(),
                    reason: reason.clone(),
                });
                if asked.any == EngineRows::Asked {
                    clear_symbols(&mut worker.tables.symbols);
                    index_kinds(&mut worker.tables.elements);
                    self.read_file(worker, visit, number, &semantic)?;
                } else {
                    let unavailable_acceptance = syntax_acceptance(&worker.tables.elements);
                    if Self::may_gate_rules(setting, unavailable_acceptance) {
                        drop(Self::gate_rules(
                            &mut worker.tables,
                            setting,
                            number,
                            unavailable_acceptance,
                        ));
                    }
                }
                match (setting.answering, asked.any) {
                    (Answering::Rows(_), _) => None,
                    (Answering::Findings, EngineRows::NotAsked) => {
                        Some(FileOutcome::Unread { reason, ran: None })
                    }
                    (Answering::Findings, EngineRows::Asked) => {
                        if semantic.is_incomplete() {
                            None
                        } else {
                            self.recorded(worker, visit, number, base).map(|ran| {
                                FileOutcome::Unread {
                                    reason,
                                    ran: Some(ran),
                                }
                            })
                        }
                    }
                }
            }
        })
    }

    /// Writes the rows the engine writes of every file, for the rules that ask for them.
    fn write_engine_rows(
        worker: &mut Worker,
        asked: EngineAsked,
        number: FileId,
        executable: ExecutablePermission,
        said: &mut Vec<ReadInPart>,
    ) -> Result<(), FileAnalysisError> {
        if asked.executable == EngineRows::Asked {
            match executable {
                ExecutablePermission::Posix(value) => {
                    worker.facts.columns_mut()?.write(FilesystemExecutableRow {
                        path: number,
                        executable: if value {
                            ryo_relations::domain::relations::types::Flag::Raised
                        } else {
                            ryo_relations::domain::relations::types::Flag::Lowered
                        },
                    })
                }
                ExecutablePermission::Unsupported => {}
                ExecutablePermission::Unknown => said.push(ReadInPart {
                    relation: Relation::FilesystemExecutable,
                    cause: PartialCause(
                        "input edge did not supply executable permission metadata".into(),
                    ),
                    lacking: ryo_relations::domain::relations::types::Missing::Any,
                    at: PartialScope::OneFile(PartialAt(number.0)),
                }),
            }
        }
        if asked.file == EngineRows::Asked {
            worker
                .facts
                .columns_mut()?
                .write(FilesystemFileRow { path: number });
        }
        if asked.lines == EngineRows::Asked {
            let lines = Integer(lines_in(BorrowedBytes(&worker.tables.bytes.0)).0);
            worker.facts.columns_mut()?.write(FilesystemLineCountRow {
                path: number,
                lines,
            });
        }
        Ok(())
    }

    /// What a file read came to, as the store keeps it: its findings, and what it shows other files: what its language noted of it,
    /// the relations it is read in part and what it hands the cross-file steps. Nothing when the run keeps nothing, or the words cannot
    /// be moved to another run.
    pub(super) fn recorded(
        &self,
        worker: &Worker,
        visit: &FileVisit<'_>,
        number: FileId,
        base: ArenaBase,
    ) -> Option<Ran> {
        let (setting, tables) = (&visit.setting, &worker.tables);
        if matches!(setting.answering, Answering::Rows(_)) {
            return None;
        }
        // A run whose store keeps nothing makes its findings alone: no words moved for another run, no digest of what it shows.
        if setting.keeping == Keeping::Discards {
            return Some(Ran {
                rows: rows_of(&tables.rendered),
                number: FileNumber(number.0),
                interface: Digest([0; 16]),
                frontier: Vec::new(),
                noted: Digest([0; 16]),
                handed: Handing::Made(Handed::default()),
            });
        }
        let moved = moved_words(tables, number, base)?;
        let graph = setting.graph.observations.as_ref();
        let (observations, slots) = match (tables.current_observations.as_deref(), graph) {
            (Some(map), Some(graph)) => {
                let count = ryo_values::domain::observations::types::ObservationReadinessCount(
                    u32::try_from(graph.frontiers.len()).ok()?,
                );
                let checked = ryo_values::domain::observations::service::validate_frontiers(
                    map,
                    graph.layout,
                    count,
                )
                .ok()?;
                (Some(checked), graph.frontiers.as_slice())
            }
            (None, Some(graph)) if !graph.frontiers.is_empty() => return None,
            (Some(_), None) => return None,
            _ => (None, &[][..]),
        };
        let mut flat = Flat::default();
        let frontier: Vec<Digest> = (0..setting.plans.len())
            .map(|rule| {
                frontier_flat(
                    &moved,
                    observations,
                    slots,
                    setting,
                    RuleIndex(rule),
                    &mut flat,
                );
                let digest = if flat.0.is_empty() {
                    Digest([0; 16])
                } else {
                    self.digest.digest_words(DigestWords(&flat.0))
                };
                flat.0.clear();
                digest
            })
            .collect();
        let noted = self.digest.digest(DigestBytes(&interface_bytes(tables).0));
        let mut shown = vec![noted];
        shown.extend(frontier.iter().copied());
        let mut ran = made_ran(tables, number, self.digest.combine(&shown), moved, base);
        ran.noted = noted;
        ran.frontier = frontier
            .iter()
            .enumerate()
            .filter(|(_, digest)| **digest != Digest([0; 16]))
            .map(|(rule, digest)| RuleFrontier {
                rule: PlacedRule(u32::try_from(rule).unwrap_or(u32::MAX)),
                digest: *digest,
            })
            .collect();
        Some(ran)
    }

    /// What the languages make of the project the manifests and the links between the files make.
    pub(crate) fn project_now(
        &self,
        input: &ProjectInput<'_>,
        symbols: &mut Symbols,
        facts: &mut dyn FactsWorker,
    ) -> Result<Projected, FileAnalysisError> {
        let mut candidates = self.language.project_entries(input);
        candidates.sort_unstable();
        candidates.dedup();
        let observed_entries = ryo_language::domain::project_context::service::observed(
            candidates
                .iter()
                .map(|path| self.corpus.observe_entry(path))
                .collect(),
        );
        let input = ProjectInput {
            observed_entries: &observed_entries,
            ..*input
        };
        let mut writer = facts.columns_mut()?;
        let answer = self.language.project(&input, symbols, &mut writer)?;
        if answer.observed_entries.as_slice()
            != ryo_language::domain::project_context::service::entries(&observed_entries)
        {
            return Err(FileAnalysisError::ProjectObservationMismatch);
        }
        let said: Vec<ReadInPart> = std::mem::take(&mut writer.partials);
        let mut projected = Projected {
            unknowns: Vec::new(),
            partials: partial_of(&said, Recovery::Whole, &[]),
            refused: answer
                .refused
                .into_iter()
                .map(|each| ManifestRefused {
                    language: each.language,
                    path: each.path,
                    reason: each.reason,
                })
                .collect(),
            unlisted: answer.unlisted,
            observed_entries: ryo_language::domain::project_context::service::receipt(
                &observed_entries,
            ),
            ..Projected::default()
        };
        self.consulted(&answer.consulted, &mut projected)?;
        Ok(projected)
    }

    /// The files the lookup read beyond the manifests, each stat'ed, read and digested as the files of the run are, in the order of
    /// `paths`, and apart from them the ones that are not there, into `into`. An unrecordable dependency fails the run.
    pub(super) fn consulted(
        &self,
        paths: &[FilePath],
        into: &mut Projected,
    ) -> Result<(), FileAnalysisError> {
        let mut bytes = FileBytes::default();
        for path in paths {
            let entry = match self.corpus.stat(path) {
                Ok(PathStat::Found(entry)) => entry,
                // A file the lookup asked for and found none of is kept as such: one that appears changes what the lookup finds.
                Ok(PathStat::Missing) => {
                    into.consulted_absent.push(path.clone());
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            self.corpus.read(&entry, &mut bytes)?;
            into.consulted.push(StoredFile {
                executable: entry.executable,
                digest: self.digest.digest(DigestBytes(&bytes.0)),
                interface: Digest([0; 16]),
                noted: Digest([0; 16]),
                size: entry.size,
                modified: entry.modified,
                path: entry.path,
            });
        }
        Ok(())
    }
}
