//! The types of the `facts` concept, the fact contract: the engine asks a language once per file for the relations the
//! file's surviving rules read, and reads the answered set after (`fact_tables`); the words those rows hold are the run's
//! base words (the language's fixed names among them) or this file's.

use crate::domain::files::types::SourcePath;
use crate::domain::node_table::types::NodeTable;
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::answers::types::AbsenceCause;
use crate::domain::fact_tables::types::{FactRequest, FactSet};
use ryo_relations::domain::relations::types::{Count, FileId, Missing};
use std::sync::Arc;

/// One file as the engine reads it: the node table, the bytes, the file's number in the run, and the language's per-file view
/// behind `FactSource`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::FileFacts;
/// use ryo_language::inbound::use_cases::walk::types::FileVisit;
///
/// struct Nodes(usize);
/// impl FileVisit for Nodes {
///     fn visit(&mut self, file: FileFacts<'_>) {
///         self.0 = ryo_language::domain::node_table::service::node_count(file.table).0;
///     }
/// }
/// let _nodes = Nodes(0);
/// ```
pub struct FileFacts<'f> {
    /// The node table, read in place.
    pub table: &'f NodeTable,
    /// The bytes of the file.
    pub source: ByteSlice<'f>,
    /// The number the run gave the file: the `File` cell of its rows and the file half of its nodes' ids.
    pub file: FileId,
    /// The file's path from the root of the run: the text a test of the file reads.
    pub path: SourcePath<'f>,
    /// The language's per-file view, filled lazily.
    pub facts: &'f mut dyn FactSource,
}

/// Names the struct alone: a language's view has no `Debug` of its own.
impl std::fmt::Debug for FileFacts<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The project's answers to the project relations, for the cross-file pass.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::{FactSource, ProjectFacts};
///
/// fn project(facts: &mut dyn FactSource) -> ProjectFacts<'_> {
///     ProjectFacts { facts }
/// }
/// let _ = project;
/// ```
pub struct ProjectFacts<'p> {
    /// The project's view.
    pub facts: &'p mut dyn FactSource,
}

/// Names the struct alone: a project's view has no `Debug` of its own.
impl std::fmt::Debug for ProjectFacts<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// How the engine asks a language for one file's facts: once, with the relations the file's surviving rules read, answered
/// as one set of tables the engine then reads like a small database. Built by `fillers::service::file_source` from the
/// language's fillers; no language writes it by hand.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{answer_of, begun, request_of};
/// use ryo_language::domain::fact_tables::types::{FactRequest, FactSet};
/// use ryo_language::domain::facts::service::{base_words, file_words};
/// use ryo_language::domain::facts::types::{FactAnswer, FactSource, FileWords};
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
/// use ryo_relations::domain::answers::types::AbsenceCause;
/// use ryo_relations::domain::relations::types::Relation;
///
/// struct Nothing;
/// impl FactSource for Nothing {
///     fn facts(&mut self, _: &FactRequest, _: &mut FileWords, _: &mut FactSet) {}
///     fn render(&mut self, _: RenderRequest) -> RenderOutcome {
///         RenderOutcome::NotApplicable
///     }
/// }
/// let mut words = file_words(&base_words(CheckedNames::default(), &[]));
/// let (request, mut set) = (request_of(&[Relation::Function]), FactSet::default());
/// begun(&mut set, &request);
/// Nothing.facts(&request, &mut words, &mut set);
/// assert_eq!(answer_of(&set, Relation::Function), FactAnswer::Refused(AbsenceCause::NotYetRead));
/// ```
pub trait FactSource {
    /// Answers the file's one request into `out`, which the engine has `begun` with it: each relation asked is written
    /// through a `fact_tables::service::writer` or refused, interning the texts it builds into `words`. A relation it
    /// leaves alone answers `Refused(NotYetRead)`. Asked once per file; nothing is asked of the file after it.
    fn facts(&mut self, request: &FactRequest, words: &mut FileWords, out: &mut FactSet);
    /// The source form of a node in a syntax role, for fixes (replaces `begin_semantics` and the retained parse).
    fn render(&mut self, request: RenderRequest) -> RenderOutcome;
}

/// The words of the run that every file shares, coded from zero: the empty text, then the language's fixed names, then the
/// constants the loaded programs compare with. Built once by `facts::service::base_words`; read-only.
///
/// ```compile_fail,E0603
/// let forged = ryo_language::domain::facts::types::BaseWords(Default::default());
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::base_words;
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let _base = base_words(CheckedNames::default(), &[NameText("print")]);
/// ```
#[derive(Clone, Debug, Default)]
pub struct BaseWords(pub(crate) Arc<BaseSet>);

/// The private body of `BaseWords`: the words, and how many of them are the language's fixed names.
#[derive(Debug, Default)]
pub(crate) struct BaseSet {
    /// The words.
    pub(crate) words: WordSet,
    /// How many fixed names the words were made with: the words after the empty text, one for each place.
    pub(crate) fixed: Count,
    /// The run's entities (modules, project symbols), interned by any worker as its files name them: an entity's id is the
    /// same in every file of the run.
    pub(crate) entities: std::sync::Mutex<WordSet>,
}

/// The file's words: the run's base words (shared, read-only) and the texts this file's fills built (cleared per file), coded
/// after the base words.
///
/// ```compile_fail,E0603
/// let forged = ryo_language::domain::facts::types::FileWords(Default::default());
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, file_words, intern, text};
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let mut words = file_words(&base_words(CheckedNames::default(), &[]));
/// let word = intern(&mut words, NameText("x"));
/// assert_eq!(text(&words, word), NameText("x"));
/// ```
#[derive(Debug, Default)]
pub struct FileWords(pub(crate) WordsEngine);

/// The private body of `FileWords`.
#[derive(Debug, Default)]
pub(crate) struct WordsEngine {
    /// The run's base words.
    pub(crate) base: BaseWords,
    /// The words this file interned, coded from the number of base words.
    pub(crate) own: WordSet,
}

/// Texts, each once, and the code of each.
#[derive(Debug, Default)]
pub(crate) struct WordSet {
    /// The texts, by code.
    pub(crate) texts: Vec<InternedText>,
    /// The code of each text.
    pub(crate) codes: hashbrown::HashMap<InternedText, CellWord>,
}

/// An entity of the run: a module or a project symbol, by the text that names it (a dotted module path, say). Interned
/// once for the run by `facts::service::entity`, so the same entity has the same id in every file and every worker: what a
/// fact keyed by a module or a project symbol is found by across files.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::EntityId;
///
/// assert!(EntityId(0) < EntityId(1));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

/// The text an entity of the run was interned by.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::EntityName;
///
/// assert_eq!(&*EntityName("pkg.module".into()).0, "pkg.module");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityName(pub Box<str>);

/// A text a word stands for, held until its words are cleared.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InternedText(pub(crate) Box<str>);

/// A text is looked up by its characters.
impl std::borrow::Borrow<str> for InternedText {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// One cell as a word: a word's code for a text cell, the stored bits of any other cell.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::CellWord;
///
/// assert!(CellWord(1) < CellWord(2));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellWord(pub u64);

/// Whether a language answered a relation for a file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::FactAnswer;
/// use ryo_relations::domain::answers::types::AbsenceCause;
///
/// assert_ne!(FactAnswer::Answered, FactAnswer::Refused(AbsenceCause::NotYetRead));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactAnswer {
    /// Every row of the relation in the file was written.
    Answered,
    /// Some rows may be missing: rules reading the relation fault in this file.
    Partial(PartialRows),
    /// The language refuses the relation for this file.
    Refused(AbsenceCause),
}

/// What a filler said of the rows it could not write: its sentence and which rows.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::{FillerSentence, PartialRows};
/// use ryo_relations::domain::relations::types::Missing;
///
/// let partial = PartialRows { cause: FillerSentence("an unread f-string".into()), missing: Missing::Any };
/// assert_eq!(partial.missing, Missing::Any);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialRows {
    /// The filler's sentence, owned: it outlives the file.
    pub cause: FillerSentence,
    /// Which rows may be missing.
    pub missing: Missing,
}

/// The sentence a filler gave for the rows it could not write.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::FillerSentence;
///
/// assert_eq!(&*FillerSentence("cannot read".into()).0, "cannot read");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FillerSentence(pub Box<str>);
