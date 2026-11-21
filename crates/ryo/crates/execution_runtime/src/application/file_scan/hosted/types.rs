//! The types of the project's rows the host hands the scan: plain data, public fields and derives.

use ryo_relations::domain::relations::types::{CellKind, FileId, Missing, Relation};
use std::collections::BTreeMap;

/// One cell of a row the host hands the scan: a word as the relation's catalog codes it, or a name as its text, which the
/// scan interns among the words of the file it is handed to.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::hosted::types::HostedCell;
///
/// assert_ne!(HostedCell::Word(1), HostedCell::Text("a".into()));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostedCell {
    /// A word as the catalog codes it: a file, a span, a number or a flag.
    Word(u64),
    /// A name's text.
    Text(Box<str>),
}

/// What a project reader said it could not read of a relation: of the whole project, or of the rows about one file.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::hosted::types::HostedPartial;
/// use ryo_relations::domain::relations::types::{Missing, Relation};
///
/// let said = HostedPartial { relation: Relation::ModuleIdentity, file: None, cause: "why".into(), missing: Missing::Any };
/// assert!(said.file.is_none());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedPartial {
    /// The relation read in part.
    pub relation: Relation,
    /// The file whose rows are affected, or none for every file.
    pub file: Option<FileId>,
    /// The reader's sentence.
    pub cause: Box<str>,
    /// Which rows may be missing.
    pub missing: Missing,
}

/// The project's rows the host hands every file's scan: the relations it answers, each one's rows by the file they are
/// about (the row's first file cell), and what the readers could not read. A relation not held is the language's.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::hosted::types::HostedRows;
///
/// assert!(HostedRows::default().held.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostedRows {
    /// The relations the host answers, sorted.
    pub held: Vec<Relation>,
    /// The rows of each held relation about each file, in the order the readers wrote them.
    pub rows: BTreeMap<(Relation, FileId), Vec<Vec<HostedCell>>>,
    /// What the readers said they could not read.
    pub partials: Vec<HostedPartial>,
}

/// One row as a project reader wrote it, before its names are given their texts: the relation, its cells' kinds and words.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::hosted::types::WrittenRow;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let row = WrittenRow { relation: Relation::ModuleIdentity, kinds: &[], words: Vec::new() };
/// assert!(row.words.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrittenRow {
    /// The relation.
    pub relation: Relation,
    /// The kind of each cell, as the catalog says it.
    pub kinds: &'static [CellKind],
    /// The words of the cells.
    pub words: Vec<u64>,
}

/// The writer a project reader is handed when the host keeps its rows for the scans: every row and every partial, kept.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::hosted::types::HostedWriter;
///
/// assert!(HostedWriter::default().rows.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostedWriter {
    /// The rows written, in order.
    pub rows: Vec<WrittenRow>,
    /// What the readers said they could not read.
    pub partials: Vec<HostedPartial>,
}
