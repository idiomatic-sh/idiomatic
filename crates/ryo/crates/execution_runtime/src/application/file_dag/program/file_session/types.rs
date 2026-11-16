//! One file session: the input it is opened with and what it answers.
use ryo_execution_domain::domain::coverage::types::Partial;
use ryo_execution_domain::domain::kind_sets::types::vocabulary::value::PreparedVocabulary;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::SemanticLookup;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use std::sync::Arc;
/// Whether a language recognized the source of a file.
#[derive(Clone, Copy, Debug)]
pub enum SourceLanguage {
    /// A language recognized the source.
    Recognized,
    /// No language recognized the source.
    Unrecognized,
}
/// The semantic session a file may be read with.
#[derive(Debug)]
pub enum SemanticSupport<'a> {
    /// The session the language began for the file.
    Supplied(&'a dyn SemanticLookup),
    /// No session: the file is read without semantics.
    Absent,
}
/// The kind vocabulary a file may be prepared with.
#[derive(Debug)]
pub enum KindSupport {
    /// The vocabulary prepared for the languages the file may be read in.
    Prepared(Arc<PreparedVocabulary>),
    /// No vocabulary: the file's kinds are not checked.
    Absent,
}
/// What the file's run observed of the producers that feed named observations.
#[derive(Debug)]
pub enum FinishedReadiness {
    /// The readiness of the producers the file's run observed.
    Observed(Arc<ryo_values::domain::records::types::FrontierReadiness>),
    /// The run observed no producer.
    Unobserved,
}
/// Output storage only; source file and rule digest come from the bound session.
#[derive(Debug)]
pub struct CaptureContext<'a> {
    /// The worker whose arena the finished rows are written into.
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    /// The arena the finished rows are written into.
    pub arena: ryo_execution_domain::domain::finish::types::Arena<'a>,
    /// Where the places of the finished rows are collected.
    pub places: &'a mut Vec<ryo_relations::domain::relations::types::NamePlace>,
    /// Where the words of the finished rows are collected.
    pub words: &'a mut ryo_relations::domain::relations::types::Words,
}
/// What a file's rules run over in one call: where its rows come from, what is read of it and how partiality is answered.
pub struct FileRunInput<'a> {
    /// Whether a language recognized the file's source.
    pub language: SourceLanguage,
    /// The file the rules run over.
    pub file: ryo_relations::domain::relations::types::FileId,
    /// Exactly one intent for every prepared rule.
    pub intents: &'a [FileRuleIntent],
    /// The source text of the file.
    pub source: ryo_values::domain::files::types::SourceView<'a>,
    /// The sources the file's rows come from, the file's tree among them.
    pub sources: crate::outbound::facts::types::fact_sources::value::FactSources<'a>,
    /// What is admitted and counts towards the file's partiality.
    pub active: &'a [ryo_relations::domain::relations::types::Relation],
    /// What core said is read in part of its own rows: the engine's rows and a failure's.
    pub said: &'a [ryo_values::domain::stored_facts::types::ReadInPart],
    /// Whether what was read is the whole file or a recovery.
    pub recovery: ryo_execution_domain::domain::coverage::types::Recovery,
    /// Whether evaluation sees the file's physical partiality or none of it.
    pub answering: Partiality,
    /// The semantic session the file may be read with.
    pub semantic: SemanticSupport<'a>,
    /// The kind vocabulary the file may be prepared with.
    pub kinds: KindSupport,
    /// The first frontier bucket of each rule, in rule order.
    pub first: &'a [ryo_values::domain::stored_facts::types::Bucket],
}
impl std::fmt::Debug for FileRunInput<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
/// Which partiality a file's rules are evaluated with; publication always sees the physical one.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::file_session::types::Partiality;
/// assert_ne!(Partiality::Physical, Partiality::Relaxed);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Partiality {
    /// Evaluation sees the file's physical partiality.
    Physical,
    /// Evaluation sees none: rows are answered as they are, partial or not.
    Relaxed,
}
/// What stopped a file's run: the engine's refusal, the worker's facts or a source of the file's rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileStop {
    /// The engine refused the file.
    Operating(OperatingError),
    /// The worker's facts refused the file.
    Facts(crate::outbound::facts::errors::FactsError),
    /// A source refused the file's rows; the reason stays with the source.
    Reader,
}
/// What a rule's run came to after its emitted root.
#[derive(Debug)]
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
#[derive(Debug)]
pub enum EmittedRoot {
    /// The rule's diagnostic is delivered: its emitted root.
    Delivered(crate::application::file_dag::program::rule_outcome::types::RootOutcome),
    /// Its diagnostic is not delivered, or the rule stopped before its root.
    Withheld,
}
/// What one rule of a file came to, in the order it ran.
#[derive(Debug)]
pub struct RanRule {
    /// The rule.
    pub rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    /// The root the rule emitted for its caller.
    pub emit: EmittedRoot,
    /// What the rule's run came to after its root.
    pub after: RuleAfter,
}
/// How a file's run ended.
#[derive(Debug)]
pub enum FileEnd {
    /// Every rule ran and the session finished.
    Finished(FinishedReadiness),
    /// A rule stopped the run, which its `RanRule` says; the session is not finished.
    Unfinished,
    /// The session refused to open or to finish.
    Refused(FileStop),
}
/// What the rules of one file came to: each rule's run, how the file ended, and the file's physical partiality.
#[derive(Debug)]
pub struct FileRules {
    /// Each rule's run, in the order the rules ran.
    pub rules: Vec<RanRule>,
    /// How the file's run ended.
    pub end: FileEnd,
    /// The relations of the file that are read in part, as the run read them.
    pub partials: Vec<Partial>,
}

/// The frontiers a rule captured for the cross-file steps, one per captured input.
#[derive(Default, Debug)]
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
