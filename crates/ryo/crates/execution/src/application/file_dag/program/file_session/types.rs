//! One file session: the input it is opened with and what it answers.
use crate::domain::coverage::types::Partial;
use crate::domain::kind_sets::types::PreparedVocabulary;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::SemanticLookup;
use crate::domain::rule_intent::types::FileRuleIntent;
use crate::outbound::facts::types::ResidentStability;
use std::sync::Arc;
/// Physical provider data; execution-owned projections and publications cannot be supplied here.
#[derive(Clone, Debug)]
pub enum SourceRows<'a> {
    Stored(ryo_relations::domain::relations::types::ColumnsView<'a>),
    Computed(&'a ryo_values::domain::rows::types::Table),
}
pub enum SourceLanguage {
    Recognized,
    Unrecognized,
}
pub struct FileInput<'a, 'facts> {
    pub language: SourceLanguage,
    pub resident_stability: ResidentStability,
    pub file: ryo_relations::domain::relations::types::FileId,
    pub evaluation: &'a [Partial],
    pub publication: &'a [Partial],
    /// Exactly one intent for every prepared rule, bound to the engine-issued generation.
    pub intents: &'a [FileRuleIntent],
    pub source: &'a [u8],
    pub elements: &'a ryo_language::domain::elements::types::Elements,
    /// The immutable provider snapshot is bound once, never replaced per rule.
    pub read: &'a dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<SourceRows<'facts>, OperatingError>,
    pub semantic: Option<&'a dyn SemanticLookup>,
    pub kinds: Option<Arc<PreparedVocabulary>>,
}
pub struct FileSession<'a, 'facts> {
    pub(in crate::application::file_dag::program) inner:
        crate::application::file_dag::program::facade_runtime::file_execution::FileRun<'a>,
    pub(in crate::application::file_dag::program) program:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    pub(in crate::application::file_dag::program) file: ryo_relations::domain::relations::types::FileId,
    pub(in crate::application::file_dag::program) elements: &'a ryo_language::domain::elements::types::Elements,
    pub(in crate::application::file_dag::program) read: &'a dyn Fn(
        ryo_relations::domain::relations::types::Relation,
    ) -> Result<SourceRows<'facts>, OperatingError>,
}
pub struct FileFinished {
    pub readiness: Option<Arc<ryo_values::domain::records::types::FrontierReadiness>>,
}
/// Output storage only; source file and rule digest come from the bound session.
pub struct CaptureContext<'a> {
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    pub arena: &'a mut Vec<u8>,
    pub places: &'a mut Vec<ryo_relations::domain::relations::types::NamePlace>,
    pub words: &'a mut ryo_relations::domain::relations::types::Words,
}
