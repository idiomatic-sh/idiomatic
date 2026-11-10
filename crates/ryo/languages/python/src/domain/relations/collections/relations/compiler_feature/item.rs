//! Project the frontend-owned module flag; no linter visitation or source-order snapshots.
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    syntax::{
        constants::{
            MODULE_FUTURE_ANNOTATIONS, MODULE_FUTURE_ANNOTATIONS_DISABLED,
            MODULE_FUTURE_ANNOTATIONS_ENABLED,
        },
        service::kind_bits,
        types::Kind,
    },
};
use ryo::languages;
/// Read one explicit Python future-annotations flag, independent of native deferred annotations.
pub(crate) struct CompilerFeatureItem;
/// Decode only the centralized syntax property vocabulary; missing/invalid remains unknown.
fn feature_flag(mode: Option<languages::NameText<'_>>) -> Option<languages::Flag> {
    match mode.map(|text| text.0) {
        Some(MODULE_FUTURE_ANNOTATIONS_ENABLED) => Some(languages::Flag::Raised),
        Some(MODULE_FUTURE_ANNOTATIONS_DISABLED) => Some(languages::Flag::Lowered),
        _ => None,
    }
}

ryo::reads! {
    CompilerFeatureItem => languages::CompilerFeature, model(Declarations) |tree, _model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        given mode = languages::property_at(tree.elements, at, MODULE_FUTURE_ANNOTATIONS);
        given feature = languages::intern_derived(symbols, tree.source, languages::NameText(super::constants::FUTURE_ANNOTATIONS));
        when feature_flag(mode) {
            Some(enabled) => {
                row { file: file, feature: value(feature), enabled: value(enabled), }
            }
            None => { partial super::constants::INCOMPLETE => languages::Missing::Any; }
        }
    }
}
