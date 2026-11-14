//! The `tree_cells` service: the cells of a tree's rows, read from the file's elements with no IO and no ports.

use super::constants::{KIND_IN_VOCABULARY, NATIVE_RELATIONS};
use crate::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_language::domain::elements::service::{
    end_at, field_name_at, kind_at, kind_names, ordinal_at, parent_at, start_at, text_at,
};
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{Flag, Integer, Position, Relation, Span, Value};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The name of the kind of the element at `at`, as the table's vocabulary names it.
///
/// # Panics
///
/// When the kind's number names no kind of the table's vocabulary: the table and its vocabulary are made together, so that is
/// a bug, never a kind with no name.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "the table's kinds are numbered in its own vocabulary; a number outside it is a bug that must crash, not answer no name"
)]
pub(crate) fn kind_name_at(elements: &Elements, at: Position) -> Symbol {
    kind_names(elements)
        .get(usize::from(kind_at(elements, at).0))
        .copied()
        .expect(KIND_IN_VOCABULARY)
}

/// The bytes the element at `at` covers.
#[must_use]
pub(crate) fn span_at(elements: &Elements, at: Position) -> Span {
    Span {
        start: start_at(elements, at),
        end: end_at(elements, at),
    }
}

/// The place of the element at `at` under its parent, as a number of a row.
#[must_use]
fn ordinal_number_at(elements: &Elements, at: Position) -> Integer {
    Integer(i64::from(ordinal_at(elements, at).0))
}

/// The row of the element at `at`: itself, its kind's name, its text, the file, its span.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
/// use ryo_language::domain::completion::service::{completion, name_kind};
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
/// use ryo_relations::domain::relations::types::{FileId, KindBits, Offset, Position, Value};
/// use ryo_relations::domain::terms::types::NameText;
/// use ryo_operators::domain::tree_cells::service::element_of;
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab"));
/// enter_node(&mut builder, KindBits(1), Offset(0));
/// token(&mut builder, KindBits(2), Offset(0), Offset(1));
/// leave_node(&mut builder, Offset(2));
/// for (bits, name) in [(1, "file"), (2, "word")] {
///     let mut names = completion(&mut elements, &mut symbols, ByteSlice(b"ab"), FileId(0));
///     name_kind(&mut names, KindBits(bits), NameText(name));
/// }
/// let tree = TreeRows { form: TreeForm::Elements, elements: &elements, file: FileId(0) };
/// assert_eq!(element_of(&tree, Position(1))[3], Value::File(FileId(0)));
/// ```
#[must_use]
pub fn element_of(tree: &TreeRows<'_>, at: Position) -> [Value; 5] {
    [0, 1, 2, 3, 4].map(|column| {
        tree_value(
            &TreeRows {
                form: TreeForm::Elements,
                ..*tree
            },
            at,
            ColumnIndex(column),
        )
    })
}

/// The edge to the element at `child`: its parent, itself, its place under the parent, its role, the file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
/// use ryo_language::domain::completion::service::{completion, name_kind};
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
/// use ryo_relations::domain::relations::types::{FileId, KindBits, Offset, Position, Value};
/// use ryo_relations::domain::terms::types::NameText;
/// use ryo_operators::domain::tree_cells::service::{edge_of, element_of};
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab"));
/// enter_node(&mut builder, KindBits(1), Offset(0));
/// token(&mut builder, KindBits(2), Offset(0), Offset(1));
/// leave_node(&mut builder, Offset(2));
/// for (bits, name) in [(1, "file"), (2, "word")] {
///     let mut names = completion(&mut elements, &mut symbols, ByteSlice(b"ab"), FileId(0));
///     name_kind(&mut names, KindBits(bits), NameText(name));
/// }
/// let tree = TreeRows { form: TreeForm::Elements, elements: &elements, file: FileId(0) };
/// assert_eq!(edge_of(&tree, Position(1))[0], element_of(&tree, Position(0))[0]);
/// ```
#[must_use]
pub fn edge_of(tree: &TreeRows<'_>, child: Position) -> [Value; 5] {
    [0, 1, 2, 3, 4].map(|column| {
        tree_value(
            &TreeRows {
                form: TreeForm::Edges,
                ..*tree
            },
            child,
            ColumnIndex(column),
        )
    })
}

/// The tree form a relation's rows are read in, for a relation the native tree answers.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::tree_cells::service::native_form;
/// use ryo_operators::domain::tree_cells::types::TreeForm;
/// use ryo_relations::domain::relations::types::Relation;
/// assert_eq!(native_form(Relation::SyntaxChild), Some(TreeForm::Edges));
/// ```
#[must_use]
pub fn native_form(relation: Relation) -> Option<TreeForm> {
    NATIVE_RELATIONS
        .iter()
        .find_map(|(native, form)| (*native == relation).then_some(*form))
}

/// One cell of the element at `at`, in the column order of the relation the tree's rows stand for; a column
/// past them is the lowered flag.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
/// use ryo_language::domain::completion::service::{completion, name_kind};
/// use ryo_language::domain::elements::types::Elements;
/// use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
/// use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
/// use ryo_relations::domain::relations::types::{FileId, KindBits, Offset, Position, Value};
/// use ryo_relations::domain::terms::types::NameText;
/// use ryo_operators::domain::tree_cells::service::{element_of, tree_value};
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
/// let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"ab"));
/// enter_node(&mut builder, KindBits(1), Offset(0));
/// token(&mut builder, KindBits(2), Offset(0), Offset(1));
/// leave_node(&mut builder, Offset(2));
/// for (bits, name) in [(1, "file"), (2, "word")] {
///     let mut names = completion(&mut elements, &mut symbols, ByteSlice(b"ab"), FileId(0));
///     name_kind(&mut names, KindBits(bits), NameText(name));
/// }
/// let tree = TreeRows { form: TreeForm::Elements, elements: &elements, file: FileId(0) };
/// assert_eq!(tree_value(&tree, Position(1), ColumnIndex(1)), element_of(&tree, Position(1))[1]);
/// ```
#[must_use]
pub fn tree_value(tree: &TreeRows<'_>, at: Position, column: ColumnIndex) -> Value {
    let elements = tree.elements;
    if tree.form == TreeForm::Edges {
        return match column.0 {
            0 => Value::Node(node_id(tree.file, parent_at(elements, at))),
            1 => Value::Node(node_id(tree.file, at)),
            2 => Value::Number(ordinal_number_at(elements, at)),
            3 => Value::Symbol(field_name_at(elements, at)),
            4 => Value::File(tree.file),
            _ => Value::Flag(Flag::Lowered),
        };
    }
    match column.0 {
        0 => Value::Node(node_id(tree.file, at)),
        1 => Value::Symbol(kind_name_at(elements, at)),
        2 => Value::Symbol(text_at(elements, at)),
        3 => Value::File(tree.file),
        4 => Value::Span(span_at(elements, at)),
        _ => Value::Flag(Flag::Lowered),
    }
}

/// Appends to `found` the nodes of `file` a tree walk from the element at `at` reaches in `direction`, in walk order: the
/// parent, the children, the ancestors nearest first, the descendants in tree order, the deeper descendants (not the
/// children), or the previous or next sibling.
pub(crate) fn walked_tree(
    elements: &Elements,
    file: ryo_relations::domain::relations::types::FileId,
    direction: ryo_values::domain::plan_parts::types::Direction,
    at: Position,
    found: &mut Vec<Value>,
) {
    use ryo_language::domain::elements::service::{
        children, next_sibling, previous_sibling, size_at,
    };
    use ryo_values::domain::plan_parts::types::Direction;
    let up = |from: Position| (from.0 != 0).then(|| parent_at(elements, from));
    let node = |position: Position| Value::Node(node_id(file, position));
    let below = (at.0 + 1..=at.0 + size_at(elements, at).0).map(Position);
    match direction {
        Direction::Parent => found.extend(up(at).map(node)),
        Direction::Child => found.extend(children(elements, at).map(node)),
        Direction::Ancestor | Direction::NearestAncestor => {
            found.extend(std::iter::successors(up(at), |each| up(*each)).map(node));
        }
        Direction::Descendant => found.extend(below.map(node)),
        Direction::Deep => {
            found.extend(
                below
                    .filter(|each| parent_at(elements, *each) != at)
                    .map(node),
            );
        }
        Direction::PreviousSibling => found.extend(previous_sibling(elements, at).map(node)),
        Direction::NextSibling => found.extend(next_sibling(elements, at).map(node)),
    }
}
