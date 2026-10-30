//! The types of the `lowering` concept: the steps of a plan that read only the tree of a file, and what each answers.

/// What a column of a lowered step holds of the element at the position of its slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ElementPart {
    /// The element's id.
    Node,
    /// The name of the element's kind.
    Kind,
    /// The text of the element.
    Text,
    /// The file.
    File,
    /// Where the element is.
    Span,
    /// The id of the element's parent, for the edge from the parent to the element.
    Parent,
    /// The element's place among its parent's children, for that edge.
    Ordinal,
    /// The role of the element under its parent, for the edge from the parent to the element.
    Role,
    /// Nothing: a column the tree does not have.
    Absent,
}

/// Which position of a row a column reads, counting from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowPosition(pub usize);

/// How many positions a row holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowWidth(pub usize);

/// Whether a column holds the id of an element, which a row can be followed from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColumnFollowing {
    /// It does.
    Follows,
    /// It holds something else.
    DoesNotFollow,
}

/// Whether a step counts the rows of a lowered step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Counting {
    /// It makes no positions, and passes its counts on as values.
    Counts,
    /// It does not.
    #[default]
    DoesNotCount,
}

/// A column of a lowered step: the slot of the row it reads, and what of that slot's element it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LoweredColumn {
    /// Which position of the row.
    pub slot: RowPosition,
    /// What of the element at that position.
    pub field: ElementPart,
}

/// What lowering a step came to: how many positions a row holds, and the columns the step answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoweredShape {
    /// How many positions a row holds.
    pub width: RowWidth,
    /// The columns, in order.
    pub columns: Vec<LoweredColumn>,
}

/// The steps of a plan that were lowered, by their place in the plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Lowering {
    /// The shape of each step, when the step is lowered.
    pub shapes: Vec<Option<LoweredShape>>,
    /// Whether the step counts the rows of a lowered step: it makes no positions, and passes its counts on as values.
    pub counting: Vec<Counting>,
}
