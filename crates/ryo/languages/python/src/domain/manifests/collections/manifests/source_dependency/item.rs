//! The `source_dependency` relation of the language: each import a Python file writes that names a module another file of the
//! repository is.

use crate::domain::links::service::{cause_of, link_span, linked_in};
use crate::domain::links::types::Linked;
use ryo::languages;

/// Answers the `source-dependency` rows of the repository's Python manifests and files.
pub(crate) struct SourceDependencyItem;

// The files are linked from the imports the parse noted of each and the packages the manifests declared. An import that names a module
// of the repository no file is makes the relation read in part at its file; an import of a module outside the repository is no row.
ryo::reads! {
    SourceDependencyItem => languages::SourceDependency, manifests(Linked) |input, symbols|
    model linked: Linked = (linked_in(input)) {
        each (&linked.unresolved) as unresolved {
            partial_at unresolved.file, cause_of(unresolved).0 => languages::Missing::Any;
        }
        each (&linked.dependencies) as link {
            row {
                source: value(link.source),
                target: value(link.target),
                span: value(link_span(link)),
                written: value(languages::intern_derived(symbols, languages::ByteSlice(&[]), languages::NameText(&link.written.0))),
            }
        }
    }
}
