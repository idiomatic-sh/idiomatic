//! Positive, platform-independent standard-library membership for real imports.
use crate::domain::file_view::types::PythonWalk;
use crate::domain::name_facts::service::imported;
use crate::domain::node::service::{node_cell, nodes_of_kind, word};
use crate::domain::node::types::Numbered;
use crate::domain::{
    imports::service::import_target,
    model::types::Declarations,
    nodes::types::Node,
    standard_library::{
        service::contains,
        types::{ModulePath, StandardLibraryMembership},
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
        when contains(model.language_target, ModulePath(text.0)) {
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

// The engine fills `standard-library-import` from the walked file: every row of the file, once.
ryo::fills! {
    StandardLibraryImportItem => languages::StandardLibraryImport, walk(PythonWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StandardLibraryImport as languages::CatalogRelation>::Row;
        let names = &mut file.buffers.text;
        for Numbered { id, .. } in nodes_of_kind(&file.syntax, Kind::Node(Node::Alias)) {
            let Some(found) = imported(&file.syntax, id, names) else {
                continue;
            };
            match contains(file.target, ModulePath(found.target.0)) {
                StandardLibraryMembership::Standard => rows.write(Row {
                    occurrence: node_cell(&file.syntax, id),
                    target: word(words, found.target),
                    file: file.syntax.file,
                }),
                StandardLibraryMembership::Other => {}
                StandardLibraryMembership::UnverifiedRelease => rows.partial(
                    languages::ReportText(super::constants::UNVERIFIED_CATALOG),
                    languages::Missing::Any,
                ),
            }
        }
    }
}
