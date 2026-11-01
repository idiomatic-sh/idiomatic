//! Retained model lifecycle support for semantic projection tests.
use super::service::visibility;
use super::types::Visibility;
use crate::domain::model::types::Declarations;
use ryo::languages::{self, ByteSlice, Tree};
/// One independent source and its expected number of enabled embeddings.
pub(super) struct ReplaySample<'a>(pub languages::NameText<'a>, pub languages::Count);
/// Read unrelated files through one allocation-retaining provider model.
pub(super) fn reparse(
    sources: &[ReplaySample<'_>],
    path: languages::SourcePath<'_>,
) -> Result<languages::Count, ()> {
    use crate::inbound::contracts::parse::contract::ParseContract;
    use crate::outbound::{notes::factory::postcard, syntax::factory::ruff_python_parser};
    use ryo::languages::Parse as _;
    use std::sync::Arc;
    let parser = ParseContract::new(
        crate::PythonParserSettings::default(),
        Arc::new(ruff_python_parser()),
        Arc::new(postcard()),
    );
    let mut elements = languages::Elements::default();
    let mut symbols = languages::Symbols::default();
    let mut model = Declarations::default();
    let mut total = 0_usize;
    for ReplaySample(source, expected) in sources {
        languages::contract::build(
            &parser,
            languages::ParseRequest {
                path,
                file: languages::FileId(0),
                bytes: ByteSlice(source.0.as_bytes()),
                notes: languages::NoteRequest::Unwanted,
                fields: languages::FieldRequest::Wanted,
            },
            &mut elements,
            &mut symbols,
        )
        .map_err(|_error| ())?;
        let tree = Tree {
            elements: &elements,
            source: ByteSlice(source.0.as_bytes()),
            file: languages::FileId(0),
        };
        parser.file_model(&tree, &mut model);
        if !model.semantic.borrow().roots.is_empty() {
            return Err(());
        }
        let count = languages::embedded_expressions(&elements)
            .filter(|embedding| {
                visibility(&tree, &model, embedding.root, &mut symbols) == Visibility::Visible
            })
            .count();
        if count != expected.0 {
            return Err(());
        }
        total += count;
    }
    Ok(languages::Count(total))
}
