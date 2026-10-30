//! The `analyzed_source` relation of the language: each file whose imports were all examined.

use crate::domain::links::service::linked_in;
use crate::domain::links::types::Linked;
use ryo::languages;

/// Answers the `analyzed-source` rows of the repository's JavaScript manifests and files.
pub(crate) struct AnalyzedSourceItem;

// Every file the parse noted is examined whole: an import it writes that names no file is the doubt of `source_dependency`.
ryo::reads! {
    AnalyzedSourceItem => languages::AnalyzedSource, manifests(Linked) |input, _symbols|
    model linked: Linked = (linked_in(input)) {
        each (&linked.analyzed) as file {
            row { path: value(*file) }
        }
    }
}
