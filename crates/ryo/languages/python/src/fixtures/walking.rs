//! Walking a source with the language's walk, for its tests and its row-equality bench: the tree the walk writes and the rows its
//! fillers write, each compared with what today's reading of the same source gives. The two number their elements differently
//! (today's table numbers nodes and tokens together in pre-order, the walk's numbers the nodes first), so an element is compared
//! by what it is and where it is: its kind, its span, and its place among the elements that share both.

use ryo::languages::{
    self, ByteSlice, CatalogRelation, CellKind, CellWord, Column, FactAnswer, FileFacts, FileId,
    FileVisit, FilledColumns, KindBits, NodeTable, Offset, Position, Relation, RelationNeeds, Walk,
    WalkRequest, facts, node_table,
};
use std::collections::HashMap;







/// How many elements the file's own tree has in the walk's table: its nodes, then its tokens; the embedded segments follow.
#[must_use]
pub(crate) fn walked_own_count(table: &NodeTable) -> usize {
    usize::from(node_table::own_end(table))
}




/// The key columns the catalog declares of a relation, in its order: each with its place in a row and what it holds. Empty for
/// a relation with no declared key.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the relations the catalog declares a key of have one; any other relation has none to ask"
)]
#[must_use]
pub(crate) fn declared_keys(relation: Relation) -> Vec<(Column, usize, CellKind)> {
    /// The declared keys of each relation named, each found among the relation's own columns.
    macro_rules! keys {
        ($( $variant:ident ),* $(,)?) => {
            match relation {
                $( Relation::$variant => <languages::$variant as CatalogRelation>::KEYS
                    .iter()
                    .filter_map(|key| {
                        let place = <languages::$variant as CatalogRelation>::columns()
                            .iter()
                            .position(|column| Column::$variant(*column) == *key)?;
                        let kind = <languages::$variant as CatalogRelation>::kinds().get(place)?;
                        Some((*key, place, *kind))
                    })
                    .collect(), )*
                _ => Vec::new(),
            }
        };
    }
    keys!(
        BindingRead,
        BindingShadow,
        ConstantKey,
        ExpressionShape,
        ExpressionType,
        Function,
        Inherits,
        LookupScope,
        RawFormatSegment,
        Reference,
        Resolves,
        Scope,
        TokenLink,
    )
}





/// A worker's words for the files `walk` lends: over the run's base words, which hold the fixed names the walk's language
/// states and no constant of a program. The fillers of a lent file write their fixed names by handle, so they are filled into
/// these words and no others.
///
/// # Errors
///
/// When the language's fixed names are refused: one is stated twice or is the empty text.
pub(crate) fn walk_words<W: Walk>(_walk: &W) -> Result<languages::FileWords, languages::FixedNamesError> {
    let names = W::vocabulary().fixed;
    Ok(facts::file_words(&facts::base_words(
        languages::fixed_names::checked(&names.0)?,
        &[],
    )))
}

/// A visit of a lent file by a function.
struct Visit<'v>(&'v mut dyn FnMut(FileFacts<'_>));

impl FileVisit for Visit<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        (self.0)(file);
    }
}

/// Walks `bytes` with `walk` as the file numbered 0, `needs` named as the relations read, and hands the lent file to `visit`.
///
/// # Errors
///
/// When the language does not walk the file.
pub(crate) fn walked<W: Walk>(
    walk: &W,
    path: &str,
    bytes: &[u8],
    needs: &[Relation],
    table: &mut NodeTable,
    scratch: &mut W::Scratch,
    visit: &mut dyn FnMut(FileFacts<'_>),
) -> Result<(), languages::WalkError> {
    walked_structured(walk, path, bytes, needs, table, scratch, None, visit)
}

/// Walks as [`walked`] does; with `structure`, the table is written `structured` (`node_table::structured`) with those
/// enclosing kind sets.
///
/// # Errors
///
/// When the language does not walk the file.
#[expect(clippy::too_many_arguments, reason = "throwaway prototype")]
pub(crate) fn walked_structured<W: Walk>(
    walk: &W,
    path: &str,
    bytes: &[u8],
    needs: &[Relation],
    table: &mut NodeTable,
    scratch: &mut W::Scratch,
    structure: Option<&[languages::KindSet]>,
    visit: &mut dyn FnMut(FileFacts<'_>),
) -> Result<(), languages::WalkError> {
    let mut builder = match structure {
        Some(enclosing) => node_table::structured(table, W::KIND_COUNT, enclosing),
        None => node_table::builder(table, W::KIND_COUNT),
    };
    walk.walk(
        WalkRequest {
            path: languages::SourcePath(path),
            file: FileId(0),
            source: ByteSlice(bytes),
            explicit: false,
        },
        RelationNeeds(needs),
        &mut builder,
        scratch,
        &mut Visit(visit),
    )
}
