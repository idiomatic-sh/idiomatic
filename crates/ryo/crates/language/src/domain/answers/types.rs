//! The types of the `answers` concept: how a language answers a relation: the reader of its trees, the answer it gives and why it gives none.
// the answer of `Support` is private on purpose: only `absent` and `reads` make one, through `From<Answer>`, which no language can call because only this crate names an `Answer`, so a reader written beside its item is no answer (the ui case `fail_support_built_without_an_item`)

use crate::domain::elements::types::Tree;
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::types::{KindBits, Position, Rows};

/// Which elements of a file's tree a relation's reader looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reads {
    /// Every node and every token.
    Every,
    /// The elements of these kinds, in the language's own numbering, and no others.
    Kinds(&'static [KindBits]),
}

/// The seals of the item traits, implemented by the declaration macro. A public path the macro expansions need, hidden: a language never names them, and a marker written by hand compiles but is a finding of `readers-in-items`.
#[doc(hidden)]
#[expect(
    clippy::inline_modules,
    reason = "the seals are a public module only the declaration macro's expansions name: as a file it would be a file of no role the hex places, for two empty traits"
)]
pub mod declared {
    /// Implemented by the declaration of a syntax relation's reader.
    pub trait Syntax {}

    /// Implemented by the declaration of a manifest or link relation's reader.
    pub trait Manifest {}
}

/// A relation read from the trees of a language, which ryo owns the shape of: the relation of ryo's catalog it writes
/// rows of, named by type, and how the language reads it. A language implements this for each relation it reads, and
/// defines no shape of its own.
///
/// Sealed against accidental impls: an impl that forgets the marker does not compile, because a hand-written reader is a reader whose rows nothing declares.
pub trait ReadItem: declared::Syntax {
    /// The relation of ryo's catalog the item writes rows of.
    type Relation: RelationsItem;

    /// What the language works out of a file once, for the reader to read: the language's `FileModel`, or `()` for an item that reads
    /// the tree alone.
    type Model: 'static;

    /// How the item reads the relation: which elements it looks at, and what it writes for each.
    const READER: Reader<Self::Relation, Self::Model>;
}

/// How a language reads the relation `R` of its files: which elements it looks at, and what it
/// writes for each, given the model the language worked out of the file (`M`).
pub struct Reader<R: RelationsItem, M: 'static> {
    /// The elements the reader looks at.
    pub reads: Reads,
    /// What it writes for an element it looks at: the rows of its relation alone, and the names it makes in the symbols.
    pub visit: &'static dyn Visiting<R, M>,
    /// Whether it also looks at the elements the language added for reading (the arguments of a macro read as expressions).
    pub expansions: Expansions,
    /// The relation the rows are of.
    pub relation: std::marker::PhantomData<R>,
}

/// What a reader does with an element it looks at: the rows of its relation alone, and the names it makes in the symbols.
pub trait Visiting<R: RelationsItem, M>: Sync {
    /// Writes the rows the element at `position` gives, into `out`.
    fn visit(
        &self,
        tree: &Tree<'_>,
        model: &M,
        symbols: &mut Symbols,
        position: Position,
        out: &mut dyn Rows<<R as RelationsItem>::Row>,
    );
}

impl<R: RelationsItem, M: 'static> std::fmt::Debug for Reader<R, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}

impl<R: RelationsItem, M: 'static> Clone for Reader<R, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: RelationsItem, M: 'static> Copy for Reader<R, M> {}

/// What a language does for a relation: it reads it, or it says the relation is not in the language. A language can say it
/// reads a relation only through the item that reads it (`reads::<Item>()`), which implements the trait ryo owns for it;
/// there is no way to make the answer from a reader written beside the item.
///
/// ```compile_fail,E0603
/// let forged = ryo_language::domain::answers::types::Support::<u8>(todo!());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Support<R>(Answer<R>);

impl<R> From<Answer<R>> for Support<R> {
    /// The support that gives `answer`: made only in this crate, which alone names an [`Answer`].
    fn from(answer: Answer<R>) -> Self {
        Self(answer)
    }
}

impl<R> AsRef<Answer<R>> for Support<R> {
    /// The answer, for the crate's engine to read.
    fn as_ref(&self) -> &Answer<R> {
        &self.0
    }
}

/// The two answers a language can give for a relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer<R> {
    /// The language gives the relation's rows through this reader.
    Reads(R),
    /// The language does not, and why.
    NotInThisLanguage(AbsenceCause),
}

/// Whether a reader also looks at the elements the language added for reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expansions {
    /// It does: the arguments of a macro read as expressions.
    Included,
    /// It does not: it reads the file's own tree.
    Ignored,
}

/// The engine-owned reader; every provider delegates to the same indexed source coordinates.
#[derive(Debug)]
pub(crate) struct SourceLocationReader<M>(
    pub std::marker::PhantomData<std::sync::atomic::AtomicPtr<M>>,
);

impl<M> declared::Syntax for SourceLocationReader<M> {}

impl<M: 'static> ReadItem for SourceLocationReader<M> {
    type Relation =
        ryo_relations::domain::relations::collections::relations::source_location::item::SourceLocation;
    type Model = M;
    const READER: Reader<
        ryo_relations::domain::relations::collections::relations::source_location::item::SourceLocation,
        M,
    > = Reader {
        reads: Reads::Every,
        visit: &Self(std::marker::PhantomData),
        expansions: Expansions::Included,
        relation: std::marker::PhantomData,
    };
}

impl<M>
    Visiting<
        ryo_relations::domain::relations::collections::relations::source_location::item::SourceLocation,
        M,
    > for SourceLocationReader<M>
{
    fn visit(
        &self,
        tree: &Tree<'_>,
        _model: &M,
        _symbols: &mut Symbols,
        position: Position,
        out: &mut dyn Rows<ryo_relations::domain::relations::collections::relations::source_location::item::SourceLocationRow>,
    ) {
        use crate::domain::elements::service::{end_at, start_at};
        use crate::domain::location::{
            service::{column, line},
            types::SourceLineOffset,
        };
        let span = ryo_relations::domain::relations::types::Span {
            start: start_at(tree.elements, position),
            end: end_at(tree.elements, position),
        };
        if span.start > span.end
            || usize::try_from(span.end.0).unwrap_or(usize::MAX) > tree.source.0.len()
        {
            return;
        }
        let index = &crate::domain::elements::service::engine(tree.elements).line_starts;
        out.write(ryo_relations::domain::relations::collections::relations::source_location::item::SourceLocationRow {
            id: ryo_relations::domain::relations::service::node_id(tree.file, position),
            file: tree.file,
            span,
            start_line: ryo_relations::domain::relations::types::Integer(line(index, SourceLineOffset(span.start.0)).0),
            end_line: ryo_relations::domain::relations::types::Integer(line(index, SourceLineOffset(span.end.0)).0),
            start_byte: ryo_relations::domain::relations::types::Integer(i64::from(span.start.0)),
            end_byte: ryo_relations::domain::relations::types::Integer(i64::from(span.end.0)),
            start_column: ryo_relations::domain::relations::types::Integer(column(index, SourceLineOffset(span.start.0)).0),
            end_column: ryo_relations::domain::relations::types::Integer(column(index, SourceLineOffset(span.end.0)).0),
        });
    }
}

/// Shared native projection used by providers without embedded expressions.
#[derive(Debug)]
pub(crate) struct NativeSemanticSyntax<M>(
    pub std::marker::PhantomData<std::sync::atomic::AtomicPtr<M>>,
);
impl<M> declared::Syntax for NativeSemanticSyntax<M> {}
impl<M: 'static> ReadItem for NativeSemanticSyntax<M> {
    type Relation =
        ryo_relations::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntax;
    type Model = M;
    const READER: Reader<Self::Relation, M> = Reader {
        reads: Reads::Every,
        visit: &Self(std::marker::PhantomData),
        expansions: Expansions::Ignored,
        relation: std::marker::PhantomData,
    };
}
impl<M>
    Visiting<
        ryo_relations::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntax,
        M,
    > for NativeSemanticSyntax<M>
{
    fn visit(
        &self,
        tree: &Tree<'_>,
        _model: &M,
        _symbols: &mut Symbols,
        position: Position,
        out: &mut dyn Rows<ryo_relations::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntaxRow>,
    ) {
        use crate::domain::elements::service::{end_at, kind_at, kind_names, start_at, text_at};
        let Some(kind) = kind_names(tree.elements)
            .get(usize::from(kind_at(tree.elements, position).0))
            .copied()
        else {
            return;
        };
        out.write(ryo_relations::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntaxRow {
            id: ryo_relations::domain::relations::service::node_id(tree.file, position),
            kind,
            text: text_at(tree.elements, position),
            file: tree.file,
            span: ryo_relations::domain::relations::types::Span { start: start_at(tree.elements, position), end: end_at(tree.elements, position) },
        });
    }
}

/// Shared native edge projection; providers with embeddings supply explicit alternatives.
#[derive(Debug)]
pub(crate) struct NativeSemanticChild<M>(
    pub std::marker::PhantomData<std::sync::atomic::AtomicPtr<M>>,
);
impl<M> declared::Syntax for NativeSemanticChild<M> {}
impl<M: 'static> ReadItem for NativeSemanticChild<M> {
    type Relation =
        ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChild;
    type Model = M;
    const READER: Reader<Self::Relation, M> = Reader {
        reads: Reads::Every,
        visit: &Self(std::marker::PhantomData),
        expansions: Expansions::Ignored,
        relation: std::marker::PhantomData,
    };
}
impl<M>
    Visiting<
        ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChild,
        M,
    > for NativeSemanticChild<M>
{
    fn visit(
        &self,
        tree: &Tree<'_>,
        _model: &M,
        _symbols: &mut Symbols,
        position: Position,
        out: &mut dyn Rows<ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChildRow>,
    ) {
        use crate::domain::elements::service::{field_name_at, ordinal_at, parent_at};
        let parent = parent_at(tree.elements, position);
        if parent == position {
            return;
        }
        out.write(ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChildRow {
            parent: ryo_relations::domain::relations::service::node_id(tree.file, parent),
            child: ryo_relations::domain::relations::service::node_id(tree.file, position),
            ordinal: ryo_relations::domain::relations::types::Integer(i64::from(ordinal_at(tree.elements, position).0)),
            field: field_name_at(tree.elements, position),
            file: tree.file,
        });
    }
}
