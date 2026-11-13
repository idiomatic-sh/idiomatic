//! One file session: the input it is opened with and what it answers.
use crate::outbound::facts::types::ResidentStability;
use ryo_execution_domain::domain::coverage::types::Partial;
use ryo_execution_domain::domain::kind_sets::types::vocabulary::value::PreparedVocabulary;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::SemanticLookup;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use std::sync::Arc;
/// Physical provider data; execution-owned projections and publications cannot be supplied here.
#[derive(Clone, Debug)]
pub enum SourceRows<'a> {
    Stored(ryo_relations::domain::relations::types::ColumnsView<'a>),
    Computed(&'a ryo_values::domain::rows::types::Table),
}
#[derive(Clone, Copy, Debug)]
pub enum SourceLanguage {
    Recognized,
    Unrecognized,
}
/// The semantic session a file may be read with.
pub enum SemanticSupport<'a> {
    Supplied(&'a dyn SemanticLookup),
    Absent,
}
/// The kind vocabulary a file may be prepared with.
pub enum KindSupport {
    Prepared(Arc<PreparedVocabulary>),
    Absent,
}
/// What the file's run observed of the producers that feed named observations.
pub enum FinishedReadiness {
    Observed(Arc<ryo_values::domain::records::types::FrontierReadiness>),
    Unobserved,
}
pub struct FileInput<'a, 'facts> {
    pub language: SourceLanguage,
    pub resident_stability: ResidentStability,
    pub file: ryo_relations::domain::relations::types::FileId,
    pub evaluation: &'a [Partial],
    pub publication: &'a [Partial],
    /// Exactly one intent for every prepared rule, bound to the engine-issued generation.
    pub intents: &'a [FileRuleIntent],
    pub source: ryo_values::domain::files::types::SourceView<'a>,
    pub elements: &'a ryo_language::domain::elements::types::Elements,
    /// The immutable provider snapshot is bound once, never replaced per rule.
    pub read: &'a dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<SourceRows<'facts>, OperatingError>,
    pub semantic: SemanticSupport<'a>,
    pub kinds: KindSupport,
}
/// The members of a file session, reachable only inside the runtime.
pub(crate) struct SessionParts<'a, 'facts> {
    pub(crate) inner:
        crate::application::file_dag::program::facade_runtime::file_execution::service::FileRun<'a>,
    pub(crate) program:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    pub(crate) file: ryo_relations::domain::relations::types::FileId,
    pub(crate) elements: &'a ryo_language::domain::elements::types::Elements,
    pub(crate) read: &'a dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<SourceRows<'facts>, OperatingError>,
}
pub struct FileFinished {
    pub readiness: FinishedReadiness,
}
/// Output storage only; source file and rule digest come from the bound session.
pub struct CaptureContext<'a> {
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    /// The arena the finished rows are written into.
    pub arena: ryo_execution_domain::domain::finish::types::Arena<'a>,
    pub places: &'a mut Vec<ryo_relations::domain::relations::types::NamePlace>,
    pub words: &'a mut ryo_relations::domain::relations::types::Words,
}
/// What a file's rules run over in one call: the session's input without the reader, which the engine binds to the worker's facts.
pub struct FileRunInput<'a> {
    pub language: SourceLanguage,
    pub file: ryo_relations::domain::relations::types::FileId,
    pub evaluation: &'a [Partial],
    pub publication: &'a [Partial],
    /// Exactly one intent for every prepared rule.
    pub intents: &'a [FileRuleIntent],
    pub source: ryo_values::domain::files::types::SourceView<'a>,
    pub elements: &'a ryo_language::domain::elements::types::Elements,
    pub semantic: SemanticSupport<'a>,
    pub kinds: KindSupport,
    /// The first frontier bucket of each rule, in rule order.
    pub first: &'a [ryo_values::domain::stored_facts::types::Bucket],
}
/// What stopped a file's run: the engine's refusal or the worker's facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileStop {
    Operating(OperatingError),
    Facts(crate::outbound::facts::errors::FactsError),
}
/// What a rule's run came to after its emitted root.
pub enum RuleAfter {
    /// Its frontiers were captured, and the kept ones kept and committed.
    Ran {
        /// The frontier captures of the file.
        captures: Vec<ryo_execution_domain::domain::frontier_capture::types::FrontierCapture>,
    },
    /// The emitted root failed, so the file stops at it.
    Unreached,
    /// The run stopped inside the rule.
    Stopped(FileStop),
}
/// The root a rule emitted for its caller.
pub enum EmittedRoot {
    /// The rule's diagnostic is delivered: its emitted root.
    Delivered(crate::application::file_dag::program::rule_outcome::types::RootOutcome),
    /// Its diagnostic is not delivered, or the rule stopped before its root.
    Withheld,
}
/// What one rule of a file came to, in the order it ran.
pub struct RanRule {
    /// The rule.
    pub rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    pub emit: EmittedRoot,
    pub after: RuleAfter,
}
/// How a file's run ended.
pub enum FileEnd {
    /// Every rule ran and the session finished.
    Finished(FinishedReadiness),
    /// A rule stopped the run, which its `RanRule` says; the session is not finished.
    Unfinished,
    /// The session refused to open or to finish.
    Refused(FileStop),
}
/// What the rules of one file came to: each rule's run, then how the file ended.
pub struct FileRules {
    pub rules: Vec<RanRule>,
    pub end: FileEnd,
}
/// Where a rule of a file runs: the rule, the file and the first frontier bucket of every rule.
#[derive(Clone, Copy)]
pub(crate) struct RuleFile<'a> {
    /// The rule.
    pub rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    pub file: ryo_relations::domain::relations::types::FileId,
    pub first: &'a [ryo_values::domain::stored_facts::types::Bucket],
}

/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
/// fn session(file: &FileSession<'_, '_>) {
///     let _ = file;
/// }
/// ```
///
/// Only its crate opens one: no field is reachable outside it.
///
/// ```compile_fail,E0616
/// use ryo_execution_runtime::application::file_dag::program::file_session::types::FileSession;
/// fn forge(handle: &FileSession<'_, '_>) {
///     let _ = &handle.parts;
/// }
/// ```
pub struct FileSession<'a, 'facts> {
    pub(crate) parts: SessionParts<'a, 'facts>,
}

/// The frontiers a rule captured for the cross-file steps, one per captured input.
#[derive(Default)]
pub struct FrontierCaptures(
    pub Vec<ryo_execution_domain::domain::frontier_capture::types::FrontierCapture>,
);

impl std::ops::Deref for FrontierCaptures {
    type Target = Vec<ryo_execution_domain::domain::frontier_capture::types::FrontierCapture>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for FrontierCaptures {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for FrontierCaptures {
    type Item = ryo_execution_domain::domain::frontier_capture::types::FrontierCapture;
    type IntoIter =
        std::vec::IntoIter<ryo_execution_domain::domain::frontier_capture::types::FrontierCapture>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a FrontierCaptures {
    type Item = &'a ryo_execution_domain::domain::frontier_capture::types::FrontierCapture;
    type IntoIter = std::slice::Iter<
        'a,
        ryo_execution_domain::domain::frontier_capture::types::FrontierCapture,
    >;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
