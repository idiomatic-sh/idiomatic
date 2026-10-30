//! The `source_dependency` relation of the language: each path a file writes that names what another file owns.

use crate::domain::links::service::{file_id, link_span, linked_in};
use crate::domain::links::types::Linked;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `source_dependency` relation's rows to.
pub(crate) struct SourceDependencyItem;

// The files are linked from what the parse left of each and what the manifests declared.
ryo::reads! {
    SourceDependencyItem => languages::SourceDependency, manifests(Linked) |input, symbols|
    model linked: Linked = (linked_in(input)) {
        each (linked.gaps.iter().zip(&linked.gap_files)) as gap_file {
            partial_at *gap_file.1, gap_file.0.to_string() => languages::Missing::Any;
        }
        each (&linked.dependencies) as link {
            row {
                source: value(file_id(link.source)),
                target: value(file_id(link.target)),
                span: value(link_span(link)),
                written: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&link.written.0))),
            }
        }
    }
}
