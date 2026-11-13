//! The interpreter's shared-walk counters of one file session, and the session its cases open.
use ryo_execution::inbound::use_cases::execution::interface::{
    FileInput, FileRuleIntent, FileSession, OperatingError, RootOutcome, RuleOutcome,
    SourceLanguage, SourceRows,
};
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::rows::types::Table;

/// How many passes the file's walk made, over how many source rows.
pub(crate) fn walked(file: &FileSession<'_, '_>) -> (u64, u64) {
    let actual = file.fixture_walk_counts();
    (actual.passes.0, actual.source_rows.0)
}
/// File `file` of `source`, recognized and stable, opened with `intents` over the tree `elements` and the reader `read`.
pub(crate) fn input<'a, 'facts>(
    file: FileId,
    source: &'a [u8],
    elements: &'a Elements,
    intents: &'a [FileRuleIntent],
    read: &'a dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> FileInput<'a, 'facts> {
    FileInput {
        language: SourceLanguage::Recognized,
        resident_stability: ResidentStability::Stable,
        file,
        evaluation: &[],
        publication: &[],
        intents,
        source: ryo_values::domain::files::types::SourceView(source),
        elements,
        read,
        semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
        kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
    }
}
/// The table a rule's root completed with.
pub(crate) fn complete(outcome: RuleOutcome) -> Table {
    match outcome.emit {
        RootOutcome::Complete(table) => table,
        RootOutcome::Failed(error) => panic!("native publication failed: {error:?}"),
        RootOutcome::Unknown(error) => panic!("native publication unknown: {error:?}"),
        RootOutcome::Partial { table, affected } => {
            panic!("native publication partial: {table:?}; affected {affected:?}")
        }
        RootOutcome::Skipped(_) => panic!("native publication was skipped"),
        RootOutcome::NoQuery => panic!("native publication has no query"),
        RootOutcome::Deferred { phase } => panic!("native publication deferred to {phase:?}"),
    }
}
/// The error a rule's root failed with; `None` when the root did not fail.
pub(crate) fn failed(outcome: RuleOutcome) -> Option<OperatingError> {
    if let RootOutcome::Failed(error) = outcome.emit {
        Some(error)
    } else {
        None
    }
}
/// The reader of a file whose SyntaxTree the tree completes: no catalog relation is read.
pub(crate) fn no_catalog(_: Relation) -> Result<SourceRows<'static>, OperatingError> {
    panic!("completed SyntaxTree is supplied by the public file input")
}
