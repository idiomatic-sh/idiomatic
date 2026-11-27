//! Prepare the first half from the same typed steps as the executor. A rule's
//! root holds file prerequisites and a necessary witness among the file's nodes.
use std::collections::HashMap;
pub(crate) const SYNTAX: [&str; 2] = ["semantic-syntax", "syntax-tree"];
pub(crate) const CHILD: [&str; 2] = ["semantic-child", "syntax-child"];

pub(crate) struct Vocab {
    pub(crate) kinds: HashMap<String, u16>,
    pub(crate) kind_names: Vec<String>,
    pub(crate) fields: HashMap<String, u16>,
    pub(crate) field_names: Vec<String>,
}

/// Columns made available by the provider in addition to its relations catalog.
pub(crate) struct WalkFact {
    pub name: &'static str,
    pub columns: &'static [(&'static str, crate::languages::CellKind)],
    pub(crate) exact: bool,
}

/// Fact availability, spelling and lexical text are supplied by the language provider.
pub(crate) struct Language {
    pub(crate) fact_family: fn(&str) -> super::pair_summary::cost::Family,
    pub(crate) walk_facts: &'static [WalkFact],
    pub(crate) facts: Vec<String>,
    pub(crate) text_kinds: Vec<u16>,
    pub(crate) host_len: fn(&str) -> Option<usize>,
    pub(crate) file_sets: Vec<String>,
    pub(crate) spelled: Vec<(String, String, Option<char>)>,
    pub(crate) is_identifier: fn(&str) -> bool,
}

/// These readers declare no necessary identifier spelling and no extra walk columns.
/// Their kind names, field names and filled relations come from the linked parser.
pub(crate) fn declaration() -> Language {
    Language {
        fact_family: |_| super::pair_summary::cost::Family::Fact,
        walk_facts: &[],
        host_len: super::finish::host::host_len,
        facts: Vec::new(),
        spelled: Vec::new(),
        text_kinds: Vec::new(),
        file_sets: Vec::new(),
        is_identifier: |_| false,
    }
}

pub(crate) fn span_bytes(table: &crate::languages::NodeTable, i: usize) -> std::ops::Range<usize> {
    crate::languages::node_table::starts(table)[i] as usize..crate::languages::node_table::ends(table)[i] as usize
}
