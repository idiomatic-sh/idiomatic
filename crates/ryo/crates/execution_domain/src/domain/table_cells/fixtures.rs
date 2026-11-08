//! The small tree the engine's tests read elements from, and the rows they run their steps over.

use crate::domain::table_cells::types::{TreeForm, TreeRows};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::kind_index::service::index_kinds;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset};
use ryo_relations::domain::terms::types::NameText;

/// A tree of four elements: a `file` node holding the tokens `word`, `other` and `word`, in this order, at positions 1 to 3.
pub struct SmallTree {
    /// The elements.
    pub elements: Elements,
    /// The names.
    pub symbols: Symbols,
}

/// The bytes the small tree is of.
pub const SMALL_SOURCE: &[u8] = b"abcd";

/// Builds the small tree.
pub fn small_tree() -> SmallTree {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(SMALL_SOURCE));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(0), Offset(1));
    token(&mut builder, KindBits(3), Offset(1), Offset(2));
    token(&mut builder, KindBits(2), Offset(2), Offset(3));
    leave_node(&mut builder, Offset(4));
    for (bits, name) in [(1, "file"), (2, "word"), (3, "other")] {
        name_kind(
            &mut completion(
                &mut elements,
                &mut symbols,
                ByteSlice(SMALL_SOURCE),
                FileId(0),
            ),
            KindBits(bits),
            NameText(name),
        );
    }
    index_kinds(&mut elements);
    SmallTree { elements, symbols }
}

/// The tree, as the rows the steps are run over.
pub fn rows_over(elements: &Elements) -> TreeRows<'_> {
    TreeRows {
        elements,
        file: FileId(0),
        form: TreeForm::Elements,
    }
}
