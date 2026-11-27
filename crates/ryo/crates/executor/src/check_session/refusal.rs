//! Why the executor refuses a query before it reads a file. The cases a caller tells apart are typed; every other refusal is its sentence.

use serde::{Deserialize, Serialize};

/// A refusal of a query, in the sentence the executor gave it before it was typed, and the cases a caller maps to its own errors.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Refusal {
    /// The query reads a parameter (or input) its request does not bind.
    MissingParameter {
        /// The parameter.
        name: String,
    },
    /// The query writes a word for a kind that no linked language has.
    UnknownKind {
        /// The word.
        word: String,
        /// The languages looked in, as a phrase.
        languages: String,
    },
    /// The query reads a relation that is neither the catalog's nor one a linked language declares.
    UnknownRelation {
        /// The name.
        name: String,
    },
    /// A language wrote output outside its file: the provider that wrote it, the fault and the relation it wrote.
    Provider {
        /// The provider's name, as the check site that saw the output knows it.
        provider: String,
        /// What was wrong with the output.
        fault: ProviderFault,
        /// The relation whose rows carry it.
        relation: String,
    },
    /// Any other refusal, as its sentence.
    Other(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingParameter { name } => write!(f, "unplanned query no caller gives {name}"),
            Self::UnknownKind { word, languages } => write!(f, "kind `{word}` names no kind of {languages}"),
            Self::UnknownRelation { name } => write!(f, "no reader declares file relation {name}"),
            Self::Provider { provider, fault, relation } => {
                write!(f, "provider {provider} wrote output outside its file: {} in {relation}", fault.word())
            }
            Self::Other(sentence) => f.write_str(sentence),
        }
    }
}

impl std::error::Error for Refusal {}

impl From<String> for Refusal {
    fn from(sentence: String) -> Self {
        Self::Other(sentence)
    }
}

impl From<&str> for Refusal {
    fn from(sentence: &str) -> Self {
        Self::Other(sentence.to_owned())
    }
}

impl From<Refusal> for String {
    fn from(refusal: Refusal) -> Self {
        refusal.to_string()
    }
}

/// A query prepared: its program, or why it is refused.
pub type Planned = Result<crate::check_session::Program, Refusal>;

/// What was wrong with the output a language wrote for a file: the four faults a provider is refused for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderFault {
    /// A span ends before it starts, or after the file ends.
    SpanOutsideFile,
    /// A node is at a place the file's table does not have.
    NodeOutsideTable,
    /// A node belongs to another file.
    NodeOfAnotherFile,
    /// A cell that names a file names one that is not the file visited.
    FileNotVisited,
}

impl ProviderFault {
    /// How the fault is worded when a refusal is shown; nothing reads it back.
    const fn word(self) -> &'static str {
        match self {
            Self::SpanOutsideFile => "span-outside-file",
            Self::NodeOutsideTable => "node-outside-table",
            Self::NodeOfAnotherFile => "node-of-another-file",
            Self::FileNotVisited => "file-not-visited",
        }
    }
}

/// Where a file's cells must lie: the positions of its table and the bytes of its source.
#[derive(Clone, Copy, Debug)]
pub struct Extent<'a> {
    /// The provider that wrote the file's rows.
    pub provider: &'a str,
    /// The file visited.
    pub home: u32,
    /// How many positions its table has.
    pub elements: u64,
    /// How many bytes it has.
    pub bytes: u64,
}

impl Extent<'_> {
    /// The fault in `cells`, the values of a column of `kind` (in the cells' own encoding), if any: one comparison for each.
    pub fn fault(&self, kind: ryo_relations::domain::relations::types::CellKind, label: &str, cells: &[u64]) -> Option<ProviderFault> {
        use ryo_relations::domain::relations::constants::{DIAGNOSTIC_BASE, NO_SPAN};
        use ryo_relations::domain::relations::types::CellKind;
        const NULL: u64 = u64::MAX;
        match kind {
            CellKind::Node => cells.iter().filter(|&&cell| cell != NULL).find_map(|&cell| {
                if cell >> 32 != u64::from(self.home) {
                    Some(ProviderFault::NodeOfAnotherFile)
                } else if u32::try_from(cell & 0xffff_ffff).is_ok_and(|place| place < DIAGNOSTIC_BASE) && (cell & 0xffff_ffff) >= self.elements {
                    Some(ProviderFault::NodeOutsideTable)
                } else {
                    None
                }
            }),
            CellKind::Span => {
                let absent = (u64::from(NO_SPAN.start.0) << 32) | u64::from(NO_SPAN.end.0);
                cells.iter().filter(|&&cell| cell != NULL && cell != absent).find_map(|&cell| {
                    let (start, end) = (cell >> 32, cell & 0xffff_ffff);
                    (end < start || end > self.bytes).then_some(ProviderFault::SpanOutsideFile)
                })
            }
            CellKind::File if label == "file" => cells.iter().filter(|&&cell| cell != NULL).find_map(|&cell| (cell != u64::from(self.home)).then_some(ProviderFault::FileNotVisited)),
            _ => None,
        }
    }
}
