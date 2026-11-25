//! Prepare the first half from the same typed steps as the executor. A rule's
//! root holds file prerequisites and a necessary witness among the file's nodes.
use std::collections::HashMap;
pub(super) const SYNTAX: [&str; 2] = ["semantic-syntax", "syntax-tree"];
pub(super) const CHILD: [&str; 2] = ["semantic-child", "syntax-child"];

pub(super) struct Vocab {
    pub(super) kinds: HashMap<String, u16>,
    pub(super) kind_names: Vec<String>,
    pub(super) fields: HashMap<String, u16>,
    pub(super) field_names: Vec<String>,
}

/// Columns made available by the provider in addition to its relations catalog.
pub(super) struct WalkFact {
    pub name: &'static str,
    pub columns: &'static [(&'static str, ryo::languages::CellKind)],
    pub exact: bool,
}

/// Fact availability, spelling and lexical text are supplied by the language provider.
pub(super) struct Language {
    pub(super) fact_family: fn(&str) -> super::pair_summary::cost::Family,
    pub(super) walk_facts: &'static [WalkFact],
    pub(super) facts: Vec<String>,
    pub(super) text_kinds: Vec<u16>,
    pub(super) host_len: fn(&str) -> Option<usize>,
    pub(super) file_sets: Vec<String>,
    pub(super) spelled: Vec<(String, String, Option<char>)>,
    pub(super) is_identifier: fn(&str) -> bool,
}

/// These readers declare no necessary identifier spelling and no extra walk columns.
/// Their kind names, field names and filled relations come from the linked parser.
pub(super) fn declaration() -> Language {
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

pub(super) fn span_bytes(table: &ryo::languages::NodeTable, i: usize) -> std::ops::Range<usize> {
    ryo::languages::node_table::starts(table)[i] as usize..ryo::languages::node_table::ends(table)[i] as usize
}
