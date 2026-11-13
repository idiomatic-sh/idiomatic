//! The support the interpreter's own cases share: the file session they open, which no case outside this directory opens.
use ryo_execution::inbound::use_cases::execution::interface::{
    FileInput, FileRuleIntent, OperatingError, SourceLanguage, SourceRows,
};
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_relations::domain::relations::types::{FileId, Relation};

/// File 0, recognized and stable, over the source `abc`, opened with `intents` over the tree `elements` and the reader `read`.
pub(crate) fn input<'a, 'facts>(
    intents: &'a [FileRuleIntent],
    elements: &'a ryo_language::domain::elements::types::Elements,
    read: &'a dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> FileInput<'a, 'facts> {
    FileInput {
        language: SourceLanguage::Recognized,
        resident_stability: ResidentStability::Stable,
        file: FileId(0),
        evaluation: &[],
        publication: &[],
        intents,
        source: ryo_values::domain::files::types::SourceView(b"abc"),
        elements,
        read,
        semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
        kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
    }
}
