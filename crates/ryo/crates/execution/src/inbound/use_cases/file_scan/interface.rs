//! What the `file_scan` use case promises its callers: a request's rule sets loaded once for the new engine, a worker's state
//! for them, and one file scanned in the worker that took it. The state a worker holds between files is behind this trait: a
//! caller keeps it and names none of its parts.
use super::types::{
    FileScanWorker, FileToScan, Noticed, RuleSetNumber, ScanLoadRequest, ScanMeasured, ScanSession,
    ScanStoredRequest, Scanned,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_runtime::application::file_scan::warning::types::{
    WarningText, WrittenStatement,
};
use ryo_language::domain::files::types::SourcePath;
use ryo_operators::spike::EngineMode;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::query_plans::types::Emit;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::scan_rows::types::ScanRow;
use ryo_values::domain::stores::types::StoredPack;

/// What `file_scan` offers its callers.
///
/// # Examples
///
/// ```
/// use ryo_execution::inbound::use_cases::file_scan::interface::FileScanUseCase;
/// use ryo_operators::spike::EngineMode;
///
/// fn runs_the_old_engine_alone(scan: &dyn FileScanUseCase) -> bool {
///     scan.mode() == EngineMode::Old
/// }
/// ```
pub trait FileScanUseCase: Send + Sync {
    /// Which engines run the files of a request.
    fn mode(&self) -> EngineMode;

    /// The rule sets of a request as the store kept them, asked before anything is compiled: under the old engine alone, none,
    /// and the store's packs are not read; otherwise one for each linked language, each read back from the stored pack of what
    /// the run was given and reported by count. Nothing when the store lacks the pack of one of them: the run compiles its
    /// rules and asks `load`.
    ///
    /// # Errors
    ///
    /// As `load`.
    fn stored(&self, request: ScanStoredRequest<'_>)
    -> Result<Option<ScanSession>, OperatingError>;

    /// The rule sets of a request, before its files: under the old engine alone, none, and nothing is loaded; otherwise one for
    /// each linked language the rules are lowered against, read back from the stored pack of what the run was given or lowered,
    /// and reported by count.
    ///
    /// # Errors
    ///
    /// `EngineHarness` when the edge composed no engine, when there is no language to lower against, and when a language whose
    /// files would be scanned is linked without its walk.
    fn load(&self, request: ScanLoadRequest<'_>) -> Result<ScanSession, OperatingError>;

    /// A worker's state for a session, before its first file.
    fn worker(&self, session: &ScanSession) -> FileScanWorker;

    /// One file, in the worker that took it: walked by the first linked language of the session that reads it, scanned by the
    /// new engine with that language's rule set, its findings moved to `rows`, after what it holds, sorted by rule, where they
    /// start and end, and cells. Nothing is done per rule. A fault is answered, never raised: the run goes on. What the
    /// engine noticed of a rule's work in the file is moved to `noticed`, after what it holds: the rule's rows are complete,
    /// and the answer is `Overran` only when the scan is set to fail on it.
    fn scan(
        &self,
        session: &ScanSession,
        worker: &mut FileScanWorker,
        file: FileToScan<'_>,
        rows: &mut Vec<ScanRow>,
        noticed: &mut Vec<Noticed>,
    ) -> Scanned;

    /// What was noticed as the warning the run prints and lists, naming the rule by its id and the file by its path, and the
    /// statement as `written` says it (where it is written; its line is counted in the documents of `query`, the rule's),
    /// or the plan step when either is none.
    fn worded(
        &self,
        noticed: &Noticed,
        rule: &RuleId,
        file: SourcePath<'_>,
        written: Option<&WrittenStatement>,
        query: Option<&QueryBundle>,
    ) -> WarningText;

    /// What the workers of a session counted over the files they scanned, together; with the session's work ledger when
    /// the run measures.
    fn measured(&self, session: &ScanSession, workers: &[&FileScanWorker]) -> ScanMeasured;

    /// What each column of a rule's rows holds in a rule set, in column order: nothing for a rule the rule set does not run.
    fn kinds<'s>(
        &self,
        session: &'s ScanSession,
        by: RuleSetNumber,
        rule: SelectedRuleOrdinal,
    ) -> &'s [CellKind];

    /// What a row of a rule emits in a rule set, as the rule set's pack says it: its subject, its key and its headings. None for
    /// a rule the rule set does not run.
    fn emit<'s>(
        &self,
        session: &'s ScanSession,
        by: RuleSetNumber,
        rule: SelectedRuleOrdinal,
    ) -> Option<&'s Emit>;

    /// The rules a rule set runs, in rule order: nothing for a rule set the session does not have.
    fn accepted<'s>(
        &self,
        session: &'s ScanSession,
        by: RuleSetNumber,
    ) -> &'s [SelectedRuleOrdinal];

    /// The packs of a session's rule sets, each under its key: what the store keeps for the next run.
    fn packs(&self, session: &ScanSession) -> Vec<StoredPack>;
}
