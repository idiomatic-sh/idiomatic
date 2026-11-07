//! The types of the `lint` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::application::file_analysis::types::FileVisit;
use crate::domain::admission::errors::AdmissionError;
use crate::domain::ordering::types::Ordered;
use crate::domain::render::types::Answering;
use crate::domain::visit::types::Wants;
use crate::outbound::lint_store::interface::ReadStore;
use ryo_execution::inbound::use_cases::metadata::interface::FeatureRows;
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes;
use ryo_execution::inbound::use_cases::metadata::interface::Unknown;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::ManifestFile;
use ryo_values::domain::files::types::{CorpusInput, Glob};
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plan_parts::types::{FlagState, LiteralText, ParameterName, WholeNumber};
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::rule_requests::types::RuleParameters;
use ryo_values::domain::runs::types::{
    AnswerShape, MemoryBytes, RuleApplicability, RuleId, Session, Skipped,
};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::Reuse;
use ryo_values::domain::stores::types::{BucketsWanted, ReadHanded, Stored, StoredRule};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Condvar, Mutex};

/// Whether the opt-in lifecycle memory diagnostics are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryDiagnostics {
    /// The diagnostics are written.
    Enabled,
    /// No capacity traversal and no diagnostic span is made.
    Disabled,
}

/// The point of a run a memory diagnostic is written at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryPhase(pub &'static str);

/// What a memory diagnostic measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryOwner(pub &'static str);

/// Which linked provider reads the sources of a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceProvider {
    /// The providers decide by each file's path, in the order they were linked.
    Recognized,
    /// The linked provider the request names reads every file.
    Named(ryo_values::domain::descriptions::types::LanguageName),
}

/// The rules to check in a session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LintRequest {
    /// The provider for these sources: the one the request names, or ordinary path recognition.
    pub source_language: SourceProvider,
    /// The session `open` returned.
    pub session: Session,
    /// Only findings in these files are reported; none means every file.
    pub files: Vec<FilePath>,
    /// The rules, each a query.
    pub rules: Vec<Rule>,
    /// The relations the host declares, with their rows: a rule names one as it names ryo's own.
    pub hosted: Vec<HostRelation>,
    /// What is written for each row a rule emits.
    pub answer: AnswerShape,
}

/// One rule: a query whose rows are violations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    /// Explicit named producer outputs consumed by this rule.
    pub observations: Vec<ryo_values::domain::observations::types::ObservationBinding>,
    /// The rule's id, named in every finding.
    pub id: RuleId,
    /// The RyoQL text.
    pub query: QueryBundle,
    /// Whether the rule runs on every parse or only files whose syntax the provider accepted.
    pub applicability: RuleApplicability,
    /// What the query's parameters are bound to.
    pub parameters: Vec<RuleParameter>,
    /// The globs of the files the rule governs; none means every file.
    pub include: Vec<Glob>,
    /// The globs of the files it does not govern.
    pub exclude: Vec<Glob>,
}

/// One parameter of a rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleParameter {
    /// The parameter's name.
    pub name: ParameterName,
    /// Its value.
    pub value: ParameterValue,
}

/// What a parameter of a rule is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParameterValue {
    /// A whole number.
    Number(WholeNumber),
    /// A yes or no.
    Flag(FlagState),
    /// Text.
    Text(LiteralText),
    /// A list or a set of numbers.
    Numbers(Vec<WholeNumber>),
    /// A list or a set of text.
    Texts(Vec<LiteralText>),
    /// A list of sets of text: the sets in order.
    TextSets(Vec<Vec<LiteralText>>),
    /// A list of tables of text, each a row.
    Tables(Vec<ParameterTable>),
}

/// One table of a `list<table>` parameter: a row, its cells by key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterTable(pub Vec<ParameterCell>);

/// One cell of a table of a `list<table>` parameter: the key it is under and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterCell {
    /// The key.
    pub key: LiteralText,
    /// The text.
    pub value: LiteralText,
}

impl From<ParameterValue> for ParameterBinding {
    fn from(value: ParameterValue) -> Self {
        match value {
            ParameterValue::Number(number) => Self::Number(number),
            ParameterValue::Flag(flag) => Self::Flag(flag),
            ParameterValue::Text(text) => Self::Text(text),
            ParameterValue::Numbers(numbers) => Self::Numbers(numbers),
            ParameterValue::Texts(texts) => Self::Texts(texts),
            ParameterValue::TextSets(sets) => Self::TextSets(sets),
            ParameterValue::Tables(tables) => Self::Tables(
                tables
                    .into_iter()
                    .map(|table| {
                        BindingTable(
                            table
                                .0
                                .into_iter()
                                .map(|cell| BindingCell {
                                    key: cell.key,
                                    value: cell.value,
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

/// What a lint answers once its findings have all been written: the files nothing read, and what the rules could not decide.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LintReport {
    /// Project dependencies observed in this run, including dependencies retained from a reused project.
    pub dependencies: Vec<ProjectDependency>,
    /// The files no linked language read, with why.
    pub unread: Vec<Unread>,
    /// What the rules could not decide, because a relation they trust the absence of rows of was read only in part.
    pub unknowns: Vec<Unknown>,
    /// Rules explicitly skipped because a file's syntax was rejected.
    pub skipped: Vec<Skipped>,
    /// How many rows each feature of the language was fed over the run, in the catalog's order: what says every feature is exercised.
    pub features: Vec<FeatureFed>,
    /// The work of each rule, in the order of the rules: the rows each of its steps was fed. A count, the same on any machine at any
    /// load, so a test can hold a rule to it. Empty unless the run counts.
    pub work: Vec<RuleWork>,
}

/// One required project lookup, whether the file was present or absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDependency {
    /// Repository-relative path; `../` may refer to a dependency above the run root.
    pub path: FilePath,
    /// The observed content identity, or the fact that no file was present.
    pub state: ProjectDependencyState,
}

/// Content state of an observed project dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectDependencyState {
    /// Opaque identity of the bytes read by the project.
    Present(ryo_values::domain::stored_facts::types::Digest),
    /// The project looked for the path and found no file.
    Missing,
}

/// The work of one rule: the rows each of its steps was fed, over the files and the cross-file finish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleWork {
    /// The rule.
    pub rule: RuleId,
    /// Its steps, in order.
    pub steps: Vec<StepFed>,
}

/// The rows one step was fed, over all its inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepFed {
    /// What the step does.
    pub operation: ryo_values::domain::query_operators::types::OperationKind,
    /// The rows of its inputs.
    pub rows: RowTotal,
}

/// How many rows a feature of the language was fed over a run, summed over the steps of the rules that have it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureFed {
    /// The feature.
    pub feature: ryo_values::domain::run_metadata::types::Feature,
    /// The rows its steps were fed: the rows of their inputs, over the files and the cross-file finish.
    pub rows: RowTotal,
}

/// Where the rules govern, and what they read.
pub(crate) struct Wanting {
    /// Which rule governs which file.
    pub placed: PlacedScopes,
    /// What the plans read of the files.
    pub wants: Wants,
}

/// The repository as this run finds it, and what the last run kept of it.
pub(crate) struct RepositoryListing {
    /// The files, in the order of their paths.
    pub files: Vec<CorpusInput>,
    /// What the last run of this build kept; empty when it kept nothing.
    pub stored: Stored,
    /// The store those tables were read from, none when the run reads none: what this run reads of it later, and keeps against, is of
    /// it alone.
    pub read: Option<Arc<dyn ReadStore>>,
    /// The most memory the run may hold.
    pub budget: MemoryBytes,
}

/// The findings waiting for their turn to leave, and the workers waiting for it.
pub(crate) struct Streaming {
    /// What waits, and how much is being read.
    pub ordered: Mutex<Ordered>,
    /// Signalled when a file's findings left or its bytes were released.
    pub turn: Condvar,
}

/// What was wrong with the output a language wrote for a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderFault {
    /// A span ends before it starts, or after the file ends.
    SpanOutsideFile,
    /// A node is at a place the file's table does not have.
    NodeOutsideTable,
    /// A node belongs to another file.
    NodeOfAnotherFile,
    /// A cell that names a file names one that is not the file visited.
    FileNotVisited,
}

/// What one of the two jobs that open a lint answered: the files under the root, or what the last run kept, each or the
/// error `E` it failed with.
#[derive(Debug)]
pub(crate) enum Listed<E> {
    /// The files under the root.
    Files(Result<Vec<CorpusInput>, E>),
    /// What the last run of this build kept, boxed: it is far larger than a list of files.
    Stored(Result<Option<Arc<dyn ReadStore>>, E>),
}

/// What the workers read together while the files are visited: the files, what every visit reads, where the findings wait for their
/// turn and how many were written.
pub(crate) struct Course<'a> {
    /// The files, in the order of their paths.
    pub files: &'a [CorpusInput],
    /// What every visit reads.
    pub visit: &'a FileVisit<'a>,
    /// Where the findings wait for their turn.
    pub streaming: &'a Streaming,
    /// Only findings in these files are written; none means every file.
    pub only: &'a [FilePath],
    /// How many findings the workers wrote.
    pub counted: &'a AtomicU64,
}

impl From<&Rule> for RuleSpec {
    fn from(rule: &Rule) -> Self {
        Self {
            id: RuleId(rule.id.0.clone()),
            query: rule.query.clone(),
            applicability: rule.applicability,
            given: rule
                .parameters
                .iter()
                .map(|parameter| Given {
                    name: parameter.name.clone(),
                    value: parameter.value.clone().into(),
                })
                .collect(),
            observations: rule.observations.clone(),
            spelled: RuleParameters(format!("{:?}", rule.parameters).into()),
            include: rule.include.clone(),
            exclude: rule.exclude.clone(),
        }
    }
}

impl From<FeatureRows> for FeatureFed {
    fn from(each: FeatureRows) -> Self {
        Self {
            feature: each.feature,
            rows: each.rows,
        }
    }
}

impl From<AdmissionError> for ProviderFault {
    fn from(error: AdmissionError) -> Self {
        match error {
            AdmissionError::SpanOutsideFile => Self::SpanOutsideFile,
            AdmissionError::NodeOutsideTable => Self::NodeOutsideTable,
            AdmissionError::NodeOfAnotherFile => Self::NodeOfAnotherFile,
            AdmissionError::FileNotVisited => Self::FileNotVisited,
        }
    }
}

/// What the stages after the reading share: the rules, the files and what the store kept.
pub(super) struct LintRun<'a> {
    /// Required file applicability for both per-file and cross-file diagnostic Emit.
    pub placed: &'a ryo_execution::inbound::use_cases::metadata::interface::PlacedScopes,
    /// The sole request-prepared executable program and its required application capabilities.
    pub program: Arc<ryo_execution::inbound::use_cases::execution::types::PreparedExecutionProgram>,
    pub services: &'a ryo_execution::inbound::use_cases::execution::types::RequestExecution,
    /// What the caller asked.
    pub request: &'a LintRequest,
    /// The rules, compiled.
    pub rules: &'a [RuleSpec],
    /// Pure rule facts retained for source demand, frontier and work accounting.
    pub metadata: &'a [ryo_values::domain::run_metadata::types::RuleRunMetadata],
    /// The rules' ids.
    pub ids: &'a [RuleId],
    /// What the rules read of the files.
    pub wants: &'a Wants,
    /// The files listed.
    pub listing: &'a RepositoryListing,
    /// The path of each file, by file number.
    pub paths: &'a [FilePath],
    /// The manifests the project reads.
    pub manifests: &'a [ManifestFile],
    /// Each worker's first bucket.
    pub first_bucket: &'a [Bucket],
    /// What each rule needs of the files.
    pub needs: &'a [Needs],
    /// How the rows the rules emit are answered.
    pub answering: &'a Answering,
    /// The rules, with their digests, as the store keeps them.
    pub stored_rules: &'a [StoredRule],
    /// Whether the files as they were and the findings of the rules that read across files are used again as the store kept them.
    pub again: Reuse,
    /// The files the store had no row of, or that the system says changed, in the order of their paths: what the store kept of them
    /// is theirs no more.
    pub read_again: &'a [FilePath],
    /// What the rules could not decide in the changed files, read before the run to see whether what they show other files changed.
    pub alone: &'a [Unknown],
    /// Rules skipped for unchanged or separately reread files.
    pub skipped: &'a [Skipped],
    /// What the files handed the last run, as far as the run read it.
    pub handed: &'a ReadHanded,
    /// The buckets of what the files handed that the run read.
    pub wanted: &'a BucketsWanted,
}

use ryo_values::domain::rule_requests::types::{Given, RuleSpec};

use ryo_values::domain::query_bindings::types::{BindingCell, BindingTable, ParameterBinding};
