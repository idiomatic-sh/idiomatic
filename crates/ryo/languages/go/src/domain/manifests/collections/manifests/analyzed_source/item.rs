//! The `analyzed_source` relation of the language: each file whose imports were all examined.

use crate::domain::links::service::linked_in;
use crate::domain::links::types::Linked;
use ryo::languages;

/// The reader the `project` contract hands the `analyzed_source` relation to, which reads its rows from the repository's manifests.
pub(crate) struct AnalyzedSourceItem;

// Every Go file the parse noted is examined whole, but one in a module whose `go.mod` was refused: an import it writes that names no
// package is the doubt of `source_dependency`.
ryo::reads! {
    AnalyzedSourceItem => languages::AnalyzedSource, manifests(Linked) |input, _symbols|
    model linked: Linked = (linked_in(input)) {
        each (&linked.analyzed) as file {
            row { path: value(*file) }
        }
    }
}
