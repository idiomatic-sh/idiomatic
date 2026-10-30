//! The `source_dependency` relation of the language: each package a file imports that is a directory of Go files of its own module,
//! one row for each file of the package.

use crate::domain::links::service::{cause_of, link_span, linked_in};
use crate::domain::links::types::Linked;
use ryo::languages;

/// The reader the `project` contract hands the `source_dependency` relation to, which reads its rows from the repository's manifests.
pub(crate) struct SourceDependencyItem;

// The files are linked from the imports the parse noted of each and the modules the manifests declared. An import under the file's
// module that names no directory of Go files, and a file whose module's `go.mod` was refused, make the relation read in part at the
// file; the standard library and another module are no row: `package_dependency` answers a module of the repository.
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
