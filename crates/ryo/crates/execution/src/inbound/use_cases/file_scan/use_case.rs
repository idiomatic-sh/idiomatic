//! Thin public orchestration over the runtime's pack load and file scan.
use super::interface::FileScanUseCase;
use super::types::{
    FileScanSetting, FileScanWorker, FileToScan, Noticed, Overruns, RuleSet, RuleSetNumber,
    ScanEngine, ScanLanguage, ScanLoadRequest, ScanMeasure, ScanMeasured, ScanSession,
    ScanStoredRequest, Scanned,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_runtime::application::file_scan::errors::FileScanError;
use ryo_execution_runtime::application::file_scan::measuring::service::ledger_lines;
use ryo_execution_runtime::application::file_scan::service::{
    counted, crossed, emit_columns, place_columns, taken, worker,
};
use ryo_execution_runtime::application::file_scan::types::{ScanMeasuring, ScanRun};
use ryo_execution_runtime::application::file_scan::warning::service::worded;
use ryo_execution_runtime::application::file_scan::warning::types::{
    NoticedStatement, SlowRule, WarningText, WrittenStatement,
};
use ryo_execution_runtime::application::pack_load::service::{harnessed, read_back};
use ryo_execution_runtime::application::pack_load::types::{
    Harness, HarnessLoad, HarnessRequest, ReadBackRequest,
};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::inbound::use_cases::walk::errors::WalkError;
use ryo_language::inbound::use_cases::walk::types::WalkRequest;
use ryo_operators::spike::{EngineMode, engine_unwalked};
use ryo_relations::domain::relations::types::{CellKind, Count};
use ryo_values::domain::query_plans::types::Emit;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::scan_rows::types::ScanRow;
use ryo_values::domain::stores::types::StoredPack;

/// Runs a request's files with the new engine the edge composed.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::interface::FileScanUseCase as _;
/// use ryo_execution::inbound::use_cases::file_scan::types::{
///     FileScanSetting, Overruns, ScanEngine, ScanLanguage, ScanMeasure,
/// };
/// use ryo_execution::inbound::use_cases::file_scan::use_case::FileScanUseCaseImpl;
/// use ryo_operators::spike::EngineMode;
/// use ryo_values::domain::descriptions::types::KindVocabularies;
/// use std::sync::Arc;
///
/// let scan = FileScanUseCaseImpl::new(FileScanSetting {
///     engine: ScanEngine::NotComposed,
///     digest: Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
///     walks: Vec::new(),
///     vocabularies: KindVocabularies(Vec::new()),
///     language: ScanLanguage::EveryLinked,
///     mode: EngineMode::Old,
///     overruns: Overruns::Report,
///     measure: ScanMeasure::Off,
/// });
/// assert_eq!(scan.mode(), EngineMode::Old);
/// ```
#[derive(Debug)]
pub struct FileScanUseCaseImpl {
    /// What the edge composed.
    setting: FileScanSetting,
}

impl FileScanUseCaseImpl {
    /// A use case over what the edge composed for the new engine.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution::inbound::use_cases::file_scan::types::FileScanSetting;
    /// use ryo_execution::inbound::use_cases::file_scan::use_case::FileScanUseCaseImpl;
    ///
    /// fn over(composed: FileScanSetting) -> FileScanUseCaseImpl {
    ///     FileScanUseCaseImpl::new(composed)
    /// }
    /// ```
    #[must_use]
    pub const fn new(setting: FileScanSetting) -> Self {
        Self { setting }
    }

    /// The rule set, when its files are scanned: every rule set under the new engine alone; under both, the ones that run a
    /// rule, as only their rows are compared.
    fn scanned<'s>(&self, set: &'s RuleSet) -> Option<&'s RuleSet> {
        (self.setting.mode == EngineMode::New || !set.load.accepted.is_empty()).then_some(set)
    }

    /// The engine the edge composed, when it composed one.
    fn engine(
        &self,
    ) -> Option<&dyn ryo_execution_runtime::outbound::engine::interface::EnginePort> {
        match &self.setting.engine {
            ScanEngine::Composed(engine) => Some(engine.as_ref()),
            ScanEngine::NotComposed => None,
        }
    }

    /// What the edge composed for the harness: none at an edge that composed no engine.
    fn harness(&self) -> Option<Harness<'_>> {
        let composed = &self.setting;
        self.engine().map(|engine| Harness {
            engine,
            digest: composed.digest.as_ref(),
            vocabularies: &composed.vocabularies,
            named: match &composed.language {
                ScanLanguage::EveryLinked => None,
                ScanLanguage::Named(name) => Some(name),
            },
        })
    }

    /// The session of the rule sets `loads` made, read back or lowered: each with its language's walk and the columns its
    /// files' rows are read and placed by, which its pack says.
    fn session(&self, loads: Vec<HarnessLoad>) -> Result<ScanSession, OperatingError> {
        let composed = &self.setting;
        let Some(engine) = self.engine() else {
            return Ok(ScanSession::default());
        };
        let mut session = ScanSession::default();
        for load in loads {
            let linked = composed
                .vocabularies
                .0
                .iter()
                .position(|each| each.language == load.language)
                .and_then(|at| composed.walks.get(at));
            let set = linked.map(|walk| RuleSet {
                walk: std::sync::Arc::clone(walk),
                columns: emit_columns(engine, &load.programs),
                places: place_columns(&load.accepted, &load.pack.emits),
                load,
            });
            // A language linked without its walk has a walk with no kind: its files cannot be scanned.
            match set {
                Some(set) if set.walk.kind_count().0 > 0 || self.scanned(&set).is_none() => {
                    session.rule_sets.push(set);
                }
                Some(set) => {
                    return Err(OperatingError::EngineHarness {
                        reason: engine_unwalked(composed.mode, &set.load.language.0),
                    });
                }
                None => {}
            }
        }
        Ok(session)
    }
}

impl FileScanUseCase for FileScanUseCaseImpl {
    fn mode(&self) -> EngineMode {
        self.setting.mode
    }

    fn stored(
        &self,
        request: ScanStoredRequest<'_>,
    ) -> Result<Option<ScanSession>, OperatingError> {
        let read = ReadBackRequest {
            given: request.given,
            selected: request.selected,
            stored: request.stored,
        };
        read_back(self.setting.mode, self.harness(), read)?
            .map(|loads| self.session(loads))
            .transpose()
    }

    fn load(&self, request: ScanLoadRequest<'_>) -> Result<ScanSession, OperatingError> {
        let loads = harnessed(
            self.setting.mode,
            self.harness(),
            HarnessRequest {
                plans: request.plans,
                bindings: request.bindings,
                statistics: request.statistics,
                given: request.given,
                stored: request.stored,
            },
        )?;
        self.session(loads)
    }

    fn worker(&self, session: &ScanSession) -> FileScanWorker {
        let Some(engine) = self.engine() else {
            return FileScanWorker::default();
        };
        let measuring = match self.setting.measure {
            ScanMeasure::Off => ScanMeasuring::Plain,
            ScanMeasure::On => ScanMeasuring::Measuring,
        };
        FileScanWorker {
            workers: session
                .rule_sets
                .iter()
                .map(|set| worker(&set.walk, engine, &set.load.programs, measuring))
                .collect(),
        }
    }

    fn scan(
        &self,
        session: &ScanSession,
        worker: &mut FileScanWorker,
        file: FileToScan<'_>,
        rows: &mut Vec<ScanRow>,
        noticed: &mut Vec<Noticed>,
    ) -> Scanned {
        let unread = FileScanError::Walk {
            cause: WalkError::Unsupported,
        };
        let Some(engine) = self.engine() else {
            return Scanned::NotWalked(crossed(&unread));
        };
        let sets = session.rule_sets.iter().zip(&mut worker.workers);
        for (number, (set, state)) in sets.enumerate() {
            let Some(set) = self.scanned(set) else {
                continue;
            };
            let run = ScanRun {
                programs: &set.load.programs,
                columns: &set.columns,
                places: &set.places,
            };
            let request = WalkRequest {
                path: file.path,
                file: file.file,
                source: file.source,
            };
            state.executable = file.executable;
            let answered = taken(engine, &run, state, request, rows);
            // What the engine noticed of a rule's work is handed over whatever the scan answered: its rows are complete.
            let kept = std::mem::take(&mut state.report.notices);
            let overran = !kept.is_empty() && self.setting.overruns == Overruns::Fail;
            noticed.extend(kept.into_iter().map(|kept| Noticed {
                rule: kept.notice.rule,
                step: kept.notice.step,
                kept,
            }));
            match answered {
                // The next linked language may read the file.
                Err(fault) if fault == unread => {}
                Err(fault) => return Scanned::Faulted(RuleSetNumber(number), crossed(&fault)),
                Ok(_) if overran => return Scanned::Overran(RuleSetNumber(number)),
                Ok(_) => return Scanned::Whole(RuleSetNumber(number)),
            }
        }
        Scanned::NotWalked(crossed(&unread))
    }

    fn worded(
        &self,
        noticed: &Noticed,
        rule: &RuleId,
        file: SourcePath<'_>,
        written: Option<&WrittenStatement>,
        query: Option<&QueryBundle>,
    ) -> WarningText {
        let statement = written
            .zip(query)
            .map_or(NoticedStatement::PlanStep, |(written, query)| {
                NoticedStatement::Written(written, query)
            });
        let slow = SlowRule {
            rule,
            file,
            statement,
        };
        worded(&noticed.kept.notice, &slow)
    }

    fn measured(&self, session: &ScanSession, workers: &[&FileScanWorker]) -> ScanMeasured {
        // The session's count is every worker's of every rule set, added up where a scan's counts are.
        let all = counted(workers.iter().flat_map(|worker| &worker.workers));
        let each = [
            all.steps,
            all.candidates,
            all.made_rows,
            all.fill_rows,
            all.visited,
        ];
        let mut measured = ScanMeasured {
            counted: each.map(|count| Count(usize::try_from(count.0).unwrap_or(usize::MAX))),
            ledger: Vec::new(),
        };
        if let (ScanMeasure::On, Some(engine)) = (self.setting.measure, self.engine()) {
            for (number, set) in session.rule_sets.iter().enumerate() {
                let of_set: Vec<_> = workers
                    .iter()
                    .filter_map(|worker| worker.workers.get(number))
                    .collect();
                let lines = ledger_lines(engine, &set.load.programs, &of_set);
                measured.ledger.extend(lines);
            }
        }
        measured
    }

    fn kinds<'s>(
        &self,
        session: &'s ScanSession,
        by: RuleSetNumber,
        rule: SelectedRuleOrdinal,
    ) -> &'s [CellKind] {
        session
            .rule_sets
            .get(by.0)
            .and_then(|set| {
                set.columns
                    .0
                    .get(usize::try_from(rule.0).unwrap_or(usize::MAX))
            })
            .map_or(&[], |columns| columns.emit.0.as_slice())
    }

    fn emit<'s>(
        &self,
        session: &'s ScanSession,
        by: RuleSetNumber,
        rule: SelectedRuleOrdinal,
    ) -> Option<&'s Emit> {
        // The pack says what each rule the rule set runs emits, in the order of those rules.
        let load = &session.rule_sets.get(by.0)?.load;
        match load.accepted.binary_search(&rule) {
            Ok(at) => load.pack.emits.get(at),
            Err(_) => None,
        }
    }

    fn accepted<'s>(
        &self,
        session: &'s ScanSession,
        by: RuleSetNumber,
    ) -> &'s [SelectedRuleOrdinal] {
        session
            .rule_sets
            .get(by.0)
            .map_or(&[], |set| set.load.accepted.as_slice())
    }

    fn packs(&self, session: &ScanSession) -> Vec<StoredPack> {
        session
            .rule_sets
            .iter()
            .map(|set| set.load.pack.clone())
            .collect()
    }
}
