//! Positive, platform-independent standard-library membership for real imports.
use crate::domain::file_view::types::PythonWalk;
use crate::domain::name_facts::service::imported;
use crate::domain::node::service::{node_cell, nodes_of_kind, word};
use crate::domain::node::types::Numbered;
use crate::domain::{nodes::types::Node, standard_library::{service::contains, types::{ModulePath, StandardLibraryMembership}}, syntax::{types::Kind}};
use ryo::languages;

/// Reads only requested import aliases; membership is attached to the current
/// file model's configured Python target, not the process running the worker.
pub(crate) struct StandardLibraryImportItem;

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
