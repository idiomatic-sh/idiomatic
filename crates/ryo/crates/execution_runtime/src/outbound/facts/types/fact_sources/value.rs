//! The sources one file's rows come from, and the one check that no two of them serve one relation.

use crate::outbound::facts::errors::FactsError;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::all;
use ryo_relations::domain::relations::types::{ColumnsView, Relation, RowWriter};
use ryo_values::domain::stored_facts::types::ReadInPart;

/// A producer of the rows of the open file: the language's reader walk, and later its semantics and the project's facts. Core
/// implements it over its language port, which knows which language read the file.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSource;
/// fn serves_any(source: &dyn FactSource) -> bool {
///     !source.serves().is_empty()
/// }
/// ```
pub trait FactSource {
    /// The relations this source makes, fixed for a request; two sources of one file never serve one relation.
    fn serves(&self) -> &[Relation];

    /// Writes the rows of `wanted`, a part of what it serves, into `out`, and answers what its readers said is read in part.
    ///
    /// # Errors
    ///
    /// `Refused` when the producer refuses what it read; the reason stays with the source.
    fn read(
        &mut self,
        symbols: &mut Symbols,
        wanted: &[Relation],
        out: &mut dyn RowWriter,
    ) -> Result<Vec<ReadInPart>, FactsError>;

    /// Admits the rows just read of `relation`, a relation it serves, against the file they were read of.
    ///
    /// # Errors
    ///
    /// `Refused` when a row does not lie within the file; the reason stays with the source.
    fn admit(&mut self, relation: Relation, rows: &ColumnsView<'_>) -> Result<(), FactsError>;
}

/// A source that also holds the file's tree: the language's reader. Core implements it.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::outbound::facts::types::fact_sources::value::TreeSource;
/// use ryo_language::domain::elements::service::end_of;
/// fn elements(source: &dyn TreeSource) -> u32 {
///     end_of(source.tree()).0
/// }
/// ```
pub trait TreeSource: FactSource {
    /// The tree the source read the file into.
    fn tree(&self) -> &Elements;
}

/// Where the file's tree comes from: the language that read it, or core's empty table for a file no language read.
enum Tree<'a> {
    /// A language read the file: its walk and its tree.
    Read(&'a mut dyn TreeSource),
    /// No language read it (failed, unsupported): core's table, empty.
    Unread(&'a Elements),
}

/// The set of sources one file's rows come from, given to the run in `FileRunInput.sources`. Core builds it for each file; whether
/// two sources serve one relation is checked once, here, so nothing downstream can meet an overlap.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSources;
/// use ryo_language::domain::elements::types::Elements;
/// let elements = Elements::default();
/// let sources = FactSources::unread(&elements);
/// let _ = sources;
/// ```
#[expect(
    missing_debug_implementations,
    reason = "it holds the file's sources as trait objects, which promise no `Debug`"
)]
pub struct FactSources<'a> {
    /// The tree and, for a file a language read, the source that read it.
    tree: Tree<'a>,
    /// The further sources, after the tree's.
    others: Vec<&'a mut dyn FactSource>,
}

impl<'a> FactSources<'a> {
    /// A file a language read: its tree source, then any further sources.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_runtime::outbound::facts::types::fact_sources::value::{FactSources, TreeSource};
    /// fn sources(tree: &mut dyn TreeSource) -> bool {
    ///     FactSources::read(tree, Vec::new()).is_ok()
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// `Overlap` naming the first relation, in catalog order, that two of the sources both serve.
    pub fn read(
        tree: &'a mut dyn TreeSource,
        others: Vec<&'a mut dyn FactSource>,
    ) -> Result<Self, FactsError> {
        if !others.is_empty() {
            let first = all().iter().find(|relation| {
                let by_tree = usize::from(tree.serves().contains(relation));
                let by_others = others
                    .iter()
                    .filter(|source| source.serves().contains(relation))
                    .count();
                by_tree + by_others > 1
            });
            if let Some(relation) = first {
                return Err(FactsError::Overlap {
                    relation: *relation,
                });
            }
        }
        Ok(Self {
            tree: Tree::Read(tree),
            others,
        })
    }

    /// A file no language read (failed, unsupported): core's empty table and no source.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_runtime::outbound::facts::types::fact_sources::value::FactSources;
    /// use ryo_language::domain::elements::types::Elements;
    /// let elements = Elements::default();
    /// let _ = FactSources::unread(&elements);
    /// ```
    #[must_use]
    pub fn unread(elements: &'a Elements) -> Self {
        Self {
            tree: Tree::Unread(elements),
            others: Vec::new(),
        }
    }

    /// The file's tree: the reader's for a read file, core's empty table for one no language read.
    pub(crate) fn tree(&self) -> &Elements {
        match &self.tree {
            Tree::Read(source) => source.tree(),
            Tree::Unread(elements) => elements,
        }
    }

    /// Every source, the tree's first, then the others in the order core gave them.
    pub(crate) fn each(&mut self) -> impl Iterator<Item = &mut (dyn FactSource + 'a)> + '_ {
        let tree: Option<&mut (dyn FactSource + 'a)> = match &mut self.tree {
            Tree::Read(source) => Some(&mut **source),
            Tree::Unread(_) => None,
        };
        tree.into_iter()
            .chain(self.others.iter_mut().map(|source| &mut **source))
    }
}
