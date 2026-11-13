//! What the `facts` port asks of the outside: the parser's tree walked once for every rule, and the rows of any other relation,
//! built the first time they are asked for and borrowed until the next file.

use super::errors::FactsError;
use super::types::{ElementRow, Kinds};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{ColumnsView, KindBits, Position, Relation};

/// What every `facts` adapter implements. `relation` takes `&self`: an adapter keeps what it built behind interior mutability, so
/// rows it lent stay borrowed while it builds another relation.
pub trait Facts {
    /// One pass over the parser's tree: each element whose kind is in `kinds`, in tree order.
    fn walk(&self, kinds: Kinds<'_>, visit: &mut dyn FnMut(Position, KindBits));

    /// The `syntax-tree` row of the element at `at`: what a root select reads of it.
    fn element(&self, at: Position) -> ElementRow;

    /// The rows of `relation` for the current file: built the first time it is asked for, then borrowed until the next file. A
    /// reader that names a text interns it in `symbols`, the file's names.
    ///
    /// # Errors
    ///
    /// `Unserved` when no adapter of the composition makes `relation`.
    fn relation(
        &self,
        relation: Relation,
        symbols: &mut Symbols,
    ) -> Result<ColumnsView<'_>, FactsError>;
}
