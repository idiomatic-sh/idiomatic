//! The types of the `facts` concept, the fact contract: the engine asks a language for a relation's rows of one file, whole or
//! the rows of one key, when an enabled program first reads them; a fill appends; the words those rows hold are the run's base
//! words (the language's fixed names among them) or this file's.

use crate::domain::files::types::SourcePath;
use crate::domain::node_table::types::NodeTable;
use crate::domain::semantics::types::{RenderOutcome, RenderRequest};
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Column, Count, FileId, Missing, Relation};
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

/// How the engine asks a language for a relation of one file. Built by `fillers::service::file_source` from the language's
/// fillers; no language writes it by hand.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, file_words};
/// use ryo_language::domain::facts::types::{FactAnswer, FactSource, FileWords, FilledColumns, Wanted};
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
/// use ryo_relations::domain::answers::types::AbsenceCause;
/// use ryo_relations::domain::relations::types::Relation;
///
/// struct Nothing;
/// impl FactSource for Nothing {
///     fn fill(&mut self, _: Relation, _: Wanted, _: &mut FileWords, _: &mut FilledColumns) -> FactAnswer {
///         FactAnswer::Refused(AbsenceCause::NotYetRead)
///     }
///     fn render(&mut self, _: RenderRequest) -> RenderOutcome {
///         RenderOutcome::NotApplicable
///     }
/// }
/// let mut words = file_words(&base_words(CheckedNames::default(), &[]));
/// let answer = Nothing.fill(Relation::Function, Wanted::Whole, &mut words, &mut FilledColumns::default());
/// assert_eq!(answer, FactAnswer::Refused(AbsenceCause::NotYetRead));
/// ```
pub trait FactSource {
    /// APPENDS to `out` the wanted rows of `relation` for this file, interning the texts it builds into `words`; never clears
    /// what earlier fills of the file wrote. Asked whole at most once per file and relation, and at most once for a key; a whole
    /// fill asked after fills by key writes every row, those keys' rows again among them, after the rows already there.
    fn fill(
        &mut self,
        relation: Relation,
        wanted: Wanted,
        words: &mut FileWords,
        out: &mut FilledColumns,
    ) -> FactAnswer;
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

/// A text a word stands for, held until its words are cleared.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InternedText(pub(crate) Box<str>);

/// A text is looked up by its characters.
impl std::borrow::Borrow<str> for InternedText {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// One relation's rows of one file, column by column, as words.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::{CellWord, FilledColumn, FilledColumns};
///
/// let columns = FilledColumns(vec![FilledColumn(vec![CellWord(7)])]);
/// assert_eq!(columns.0.len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct FilledColumns(pub Vec<FilledColumn>);

/// One column of filled rows.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::{CellWord, FilledColumn};
///
/// assert_eq!(FilledColumn(vec![CellWord(1), CellWord(2)]).0.len(), 2);
/// ```
#[derive(Debug, Default)]
pub struct FilledColumn(pub Vec<CellWord>);

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
/// use ryo_language::domain::facts::types::{FactAnswer, FillExtent};
///
/// assert_ne!(FactAnswer::Answered(FillExtent::Asked), FactAnswer::Answered(FillExtent::Whole));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactAnswer {
    /// The rows were written: the rows asked for, or every row of the file where a key was asked.
    Answered(FillExtent),
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

/// Which rows of a relation a fill is asked for.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::{CellWord, Wanted};
/// use ryo_relations::domain::relations::collections::relations::token_link::item::TokenLinkColumn;
/// use ryo_relations::domain::relations::types::Column;
///
/// let by_source = Wanted::Key(Column::TokenLink(TokenLinkColumn::Source), CellWord(7));
/// assert_ne!(by_source, Wanted::Whole);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wanted {
    /// Every row of the file.
    Whole,
    /// The rows whose column holds the cell: a node, a number or a word of the file.
    Key(Column, CellWord),
}

/// Which rows a fill wrote.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::types::FillExtent;
///
/// assert_ne!(FillExtent::Asked, FillExtent::Whole);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillExtent {
    /// The rows it was asked for.
    Asked,
    /// Every row of the file, though a key was asked: the language has no body for that key. Legal only for a column the
    /// relation's catalog item does not declare among its `keys`.
    Whole,
}
