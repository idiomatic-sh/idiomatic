//! The types of the `regrouping` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::types::Operation;

/// The two joins a join after the files is regrouped into.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Regrouped {
    /// The join of the two per-file inputs, made in each file.
    pub inner: Operation,
    /// The join of the other input with it, after the files, reading `inner` at the place given to it.
    pub outer: Operation,
}

/// A join after the files read as one input from after the files and one per-file input.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SplitJoin<'a> {
    /// The input from after the files.
    pub crossing: Input,
    /// The per-file input.
    pub local: Input,
    /// The side the input from after the files is on.
    pub crossing_side: Side,
    /// The key columns of the input from after the files, in the join's own numbering of that side.
    pub crossing_keys: &'a [ColumnIndex],
    /// The key columns of the per-file input, as columns of that input.
    pub local_keys: Vec<ColumnIndex>,
    /// The view the join reads the per-file input through: the join's own when that input is on the right, else none.
    pub local_view: &'a [ColumnIndex],
    /// The columns of the join's answer.
    pub output: &'a [ryo_values::domain::plan_parts::types::JoinColumn],
    /// The view of the right input, which stays with the input from after the files when it is on the right.
    pub view: &'a [ColumnIndex],
}
