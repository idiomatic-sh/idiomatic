//! A parser implementation change invalidates completed active results.
use super::fixtures::{ParsedGrammar, RejectedGrammar};
use crate::domain::format_syntax::types::FormatParseResult;

#[test]
fn completed_active_results_are_invalidated_when_parser_implementation_changes() {
    use crate::domain::{format_parsing::service::prepare, model::types::Declarations};
    use crate::fixtures::reading::{initialize_model, parsed};
    use ryo::languages::{self, ByteSlice, Symbols, Tree};
    let file = parsed(b"value = 'same input'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    let mut symbols = Symbols::default();
    initialize_model(&tree, &mut model);
    assert!(matches!(
        prepare::<ParsedGrammar>(
            &tree,
            &model.format_parsing,
            &mut symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| (crate::domain::semantic_syntax::service::visibility(
                &tree, &model, at, symbols
            ) == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        )
        .values[0]
            .result,
        FormatParseResult::Parsed(_)
    ));
    assert!(matches!(
        prepare::<RejectedGrammar>(
            &tree,
            &model.format_parsing,
            &mut symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| (crate::domain::semantic_syntax::service::visibility(
                &tree, &model, at, symbols
            ) == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        )
        .values[0]
            .result,
        FormatParseResult::Malformed(_)
    ));
}
