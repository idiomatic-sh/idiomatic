//! The types of the `tree_walk` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::operating::types::Remembering;
use crate::domain::table_cells::types::TreeRows;
use crate::domain::table_scratch::types::{Candidates, ElementPlaces, Marks};
use crate::domain::walks::types::FilterScratch;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, JoinColumn};

/// What walking the tree of a file from the starts works with, taken out of the scratch while the walk runs.
pub(crate) struct TreeWalk<'x, 'o> {
    /// The tree walked.
    pub tree: TreeRows<'x>,
    /// The elements the answers are among, when the domain is the elements of the tree.
    pub among: Option<TreeRows<'x>>,
    /// Which way the walk goes.
    pub direction: Direction,
    /// The column of a start row that holds the entity to start from.
    pub id: ColumnIndex,
    /// The columns of the answer, or none for the start with the entity found.
    pub output: &'o [JoinColumn],
    /// Whether the nearest ancestors are remembered for the next start.
    pub remembering: Remembering,
    /// Where each entity of the tree is, among those the domain lists.
    pub marks: Marks,
    /// The places the walk from one start reaches.
    pub found: Candidates,
    /// The nearest ancestor of each element, once known.
    pub nearest: ElementPlaces,
    /// The elements climbed through while the nearest ancestor of a start is looked for.
    pub climbing: ElementPlaces,
    /// What deciding an element of the domain works with.
    pub filter: FilterScratch<'o>,
}
