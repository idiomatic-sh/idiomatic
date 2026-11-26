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
    /// Any other refusal, as its sentence.
    Other(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingParameter { name } => write!(f, "unplanned query no caller gives {name}"),
            Self::UnknownKind { word, languages } => write!(f, "kind `{word}` names no kind of {languages}"),
            Self::UnknownRelation { name } => write!(f, "no reader declares file relation {name}"),
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
