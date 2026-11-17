//! The `Activity` value: whether a node runs in the open file, the engine's answer for each node before the runner reads any
//! of its inputs, and the layout a dormant node holds its empty table in.
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::rows::types::Table;

/// Whether a node runs in the open file, answered by the engine before the runner reads any of its inputs.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::activity::value::{Activity, NodeLayout};
/// assert_ne!(Activity::Active, Activity::Dormant(NodeLayout(&[])));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity<'a> {
    /// It runs.
    Active,
    /// It can have neither a row nor a row error in the file: it holds the empty table of this layout and reads no input.
    Dormant(NodeLayout<'a>),
}

/// The columns a node's rows answer with.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::activity::value::NodeLayout;
/// assert!(NodeLayout(&[]).0.is_empty());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeLayout<'a>(pub &'a [CellKind]);

impl NodeLayout<'_> {
    /// Empties `rows` into this layout, keeping its room: the table of a node with no row.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::activity::value::NodeLayout;
    /// use ryo_relations::domain::relations::types::CellKind;
    /// let mut rows = ryo_values::domain::rows::types::Table::default();
    /// NodeLayout(&[CellKind::Node]).emptied(&mut rows);
    /// assert_eq!(rows.kinds, [CellKind::Node]);
    /// ```
    pub fn emptied(self, rows: &mut Table) {
        ryo_values::domain::rows::service::reset(rows, self.0);
    }
}
