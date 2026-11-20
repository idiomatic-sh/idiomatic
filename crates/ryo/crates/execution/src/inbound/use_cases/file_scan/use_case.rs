//! Thin public orchestration over the runtime's pack load and file scan.
use super::interface::FileScanUseCase;
use super::types::{
    FileScanSetting, FileScanWorker, FileToScan, RuleSet, RuleSetNumber, ScanEngine, ScanLanguage,
    ScanLoadRequest, ScanSession, Scanned,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_runtime::application::file_scan::errors::FileScanError;
use ryo_execution_runtime::application::file_scan::service::{
    crossed, emit_columns, place_columns, taken, worker,
};
use ryo_execution_runtime::application::file_scan::types::ScanRun;
use ryo_execution_runtime::application::pack_load::service::harnessed;
use ryo_execution_runtime::application::pack_load::types::{Harness, HarnessRequest};
use ryo_language::inbound::use_cases::walk::errors::WalkError;
use ryo_language::inbound::use_cases::walk::types::WalkRequest;
use ryo_operators::spike::{EngineMode, engine_unwalked};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::scan_rows::types::ScanRow;
use ryo_values::domain::stores::types::StoredPack;

/// Runs a request's files with the new engine the edge composed.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::interface::FileScanUseCase as _;
/// use ryo_execution::inbound::use_cases::file_scan::types::{FileScanSetting, ScanEngine, ScanLanguage};
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
}

impl FileScanUseCase for FileScanUseCaseImpl {
    fn mode(&self) -> EngineMode {
        self.setting.mode
    }

    fn load(&self, request: ScanLoadRequest<'_>) -> Result<ScanSession, OperatingError> {
        let composed = &self.setting;
        let loads = harnessed(
            composed.mode,
            self.engine().map(|engine| Harness {
                engine,
                digest: composed.digest.as_ref(),
                vocabularies: &composed.vocabularies,
                named: match &composed.language {
                    ScanLanguage::EveryLinked => None,
                    ScanLanguage::Named(name) => Some(name),
                },
            }),
            HarnessRequest {
                plans: request.plans,
                digests: request.digests,
                stored: request.stored,
            },
        )?;
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
                places: place_columns(request.plans),
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

    fn worker(&self, session: &ScanSession) -> FileScanWorker {
        let Some(engine) = self.engine() else {
            return FileScanWorker::default();
        };
        FileScanWorker {
            workers: session
                .rule_sets
                .iter()
                .map(|set| worker(&set.walk, engine, &set.load.programs))
                .collect(),
        }
    }

    fn scan(
        &self,
        session: &ScanSession,
        worker: &mut FileScanWorker,
        file: FileToScan<'_>,
        rows: &mut Vec<ScanRow>,
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
            match taken(engine, &run, state, request, rows) {
                // The next linked language may read the file.
                Err(fault) if fault == unread => {}
                Err(fault) => return Scanned::Faulted(RuleSetNumber(number), crossed(&fault)),
                Ok(_) => return Scanned::Whole(RuleSetNumber(number)),
            }
        }
        Scanned::NotWalked(crossed(&unread))
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
