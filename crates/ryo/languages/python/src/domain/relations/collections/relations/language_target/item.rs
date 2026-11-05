//! Write the configured Python target independently of syntax recovery.

use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;

/// Reads the parser target configured for the current source.
pub(crate) struct LanguageTargetItem;

ryo::reads! {
    LanguageTargetItem => languages::LanguageTarget, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Module)] as kind_bits,
        each (std::iter::once(model.language_target)) as target {
            given major = languages::Integer(target.0);
            given minor = languages::Integer(target.1);
            row { file: file, major: value(major), minor: value(minor), }
        }
    }
}
