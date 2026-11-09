//! Classify using the engine-prepared model, then delegate eligible carrier parsing to the syntax port, when the request's demand
//! reads the expanded annotations.
use crate::{
    domain::{model::types::Declarations, semantic_syntax::service::completion_hosts},
    outbound::syntax::{
        interface::SyntaxPort,
        types::{DeferredAnnotation, SourceText},
    },
};
use ryo::languages;

use super::constants::MODEL_PROMISED;

/// The model is an explicit input promised by Parse::completion_requirement.
#[expect(
    clippy::expect_used,
    reason = "the common engine prepares exactly Parse::FileModel before this provider's completion; absence is a programmer contract violation and must not become false semantic absence"
)]
pub(crate) fn expand(
    source: SourceText<'_>,
    completion: &mut languages::Completion<'_>,
    syntax: &dyn SyntaxPort,
) {
    // No wanted reader reads the expanded annotations: none are parsed, and no model is prepared for them.
    if languages::completion_requirement(completion)
        != languages::CompletionRequirement::PreparedModel
    {
        return;
    }
    let carriers =
        languages::with_file_model::<Declarations, _>(completion, |tree, symbols, model| {
            completion_hosts(tree, model, symbols)
                .into_iter()
                .map(|host| DeferredAnnotation {
                    host,
                    start: languages::start_at(tree.elements, host),
                    end: languages::end_at(tree.elements, host),
                })
                .collect::<Vec<_>>()
        })
        .expect(MODEL_PROMISED);
    if !carriers.is_empty() {
        languages::append_embedded(completion, |builder| {
            syntax.expand_annotations(source, &carriers, builder)
        });
    }
}
