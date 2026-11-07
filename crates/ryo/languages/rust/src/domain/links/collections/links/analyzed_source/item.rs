//! The `analyzed_source` relation of the language: each source file whose written paths were all examined.

use crate::domain::links::service::{file_id, linked_in};
use crate::domain::links::types::Linked;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `analyzed_source` relation's rows to.
pub(crate) struct AnalyzedSourceItem;

// A file no crate root reaches is truly not analyzed: its absence from the rows is the answer, not a doubt. The files are linked from what the parse left of each and what the manifests declared.
ryo::reads! {
    AnalyzedSourceItem => languages::AnalyzedSource, manifests(Linked) |input, _symbols|
    model linked: Linked = (linked_in(input)) {
        each (linked.gaps.iter().zip(&linked.gap_files)) as gap_file {
            partial_at *gap_file.1, gap_file.0.to_string() => languages::Missing::Any;
        }
        each (&linked.analyzed) as place {
            row { path: value(file_id(*place)) }
        }
    }
}
