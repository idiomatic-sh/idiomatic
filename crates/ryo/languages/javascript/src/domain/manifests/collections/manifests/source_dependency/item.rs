//! The `source_dependency` relation of the language: each module a file imports that is another file of the repository.

use crate::domain::links::service::{cause_of, link_span, linked_in};
use crate::domain::links::types::Linked;
use ryo::languages;

/// Answers the `source-dependency` rows of the repository's JavaScript manifests and files.
pub(crate) struct SourceDependencyItem;

// The files are linked from the imports the parse noted of each. An import that names a file of the repository no file is makes the
// relation read in part at its file; a package, of the repository or outside it, is no row: `package_dependency` answers it.
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
