//! Positive, platform-independent standard-library membership for real imports.
use crate::domain::{
    imports::service::import_target,
    model::types::Declarations,
    nodes::types::Node,
    standard_library::{
        service::contains,
        types::{ModulePath, PythonMajor, PythonMinor, StandardLibraryMembership, TargetVersion},
    },
    syntax::{service::kind_bits, types::Kind},
};
use ryo::languages;

/// Reads only requested import aliases; membership is attached to the current
/// file model's configured Python target, not the process running the worker.
pub(crate) struct StandardLibraryImportItem;

ryo::reads! {
    StandardLibraryImportItem => languages::StandardLibraryImport, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Alias)] as kind_bits,
        given target = import_target(tree, symbols, at)?;
        given text = languages::text_of(symbols, tree.source, target)?;
        when contains(TargetVersion { major: PythonMajor(model.language_target.0), minor: PythonMinor(model.language_target.1) }, ModulePath(text.0)) {
            StandardLibraryMembership::Standard => {
                row {
                    occurrence: node(at),
                    target: value(target),
                    file: file,
                }
            }
            StandardLibraryMembership::Other => {}
            StandardLibraryMembership::UnverifiedRelease => {
                partial super::constants::UNVERIFIED_CATALOG => languages::Missing::Any;
            }
        }
    }
}
