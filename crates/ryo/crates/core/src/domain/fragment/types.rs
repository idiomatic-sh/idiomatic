//! The types of the `fragment` concept: the steps of a plan that read only the tree of a file, run over the positions of its
//! elements and not over rows of values.

use crate::domain::lowering::types::Lowering;
use crate::domain::positions::types::LoweredRoom;
use crate::domain::table_cells::types::TreeRows;
use ryo_values::domain::plans::types::Plan;

/// The rows of the lowered steps for the file being run, and the room the steps work in.
#[derive(Debug)]
pub(crate) struct FragmentRun<'a, 'f> {
    /// The plan.
    pub plan: &'a Plan,
    /// What was lowered of it.
    pub lowering: &'a Lowering,
    /// The tree of the file.
    pub tree: TreeRows<'f>,
    /// What the steps keep from file to file.
    pub room: &'a mut LoweredRoom,
}
