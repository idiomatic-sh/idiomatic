//! The constants of the `costing` part: the fixed priors a load with no statistics orders accesses by (scope-engine3 2.2 row
//! 18). Each is an estimate; none is a bound, and none refuses anything. The sizes are of one file; the rows an access
//! gives, the shares a test keeps and the ranks a test waits for are the cost model's (`swarm/cut`), in 1,024ths.

use super::types::PlanEstimate;
use crate::domain::lowering::definitions::types::RowEstimate;

/// The elements of a file's node table.
pub(crate) const ELEMENTS: RowEstimate = RowEstimate(2_048);
/// The rows of a filled relation.
pub(crate) const FILLED_ROWS: RowEstimate = RowEstimate(512);
/// The rows of a group table whose input nothing sizes.
pub(crate) const GROUP_ROWS: RowEstimate = RowEstimate(64);
/// A literal with no counts is sized at one row in this many of its source.
pub(crate) const LITERAL_SHARE: RowEstimate = RowEstimate(256);
/// A kind's ids are one element in this many.
pub(crate) const KIND_SHARE: RowEstimate = RowEstimate(32);
/// The anchors tried for a branch that has no given cell: the cheapest of this many.
pub(crate) const ANCHORS_TRIED: usize = 4;
/// The greatest estimate: every estimate past it is it.
pub(crate) const ESTIMATE_CAP: RowEstimate = RowEstimate(1 << 40);

/// One row, or one step.
pub(crate) const ONE: PlanEstimate = PlanEstimate(1_024);
/// The greatest estimate of the planner: `ESTIMATE_CAP` rows.
pub(crate) const PLAN_CAP: PlanEstimate = PlanEstimate(1 << 50);
/// The least rows an access is taken to leave of a candidate: a twentieth.
pub(crate) const LEAST_ROWS: PlanEstimate = PlanEstimate(51);
/// The children of one parent.
pub(crate) const CHILDREN: PlanEstimate = PlanEstimate(4 * 1_024);
/// The children of one parent in a field that is given: one and a half.
pub(crate) const FIELD_CHILDREN: PlanEstimate = PlanEstimate(1_536);
/// The rows of a relation's own key that holds any number of rows per value.
pub(crate) const MANY_ROWS: PlanEstimate = PlanEstimate(2 * 1_024);
/// The rows a filled relation holds for a value of a column that is not its key and holds an element: 1.3.
pub(crate) const ELEMENT_COLUMN_ROWS: PlanEstimate = PlanEstimate(1_331);
/// The rows a filled relation holds for a value of a column that is not its key and holds no element.
pub(crate) const VALUE_COLUMN_ROWS: PlanEstimate = PlanEstimate(6 * 1_024);
/// A key of several columns reaches just under what its most selective column does: 0.99 of it.
pub(crate) const SEVERAL_KEYS_SHARE: PlanEstimate = PlanEstimate(1_014);
/// The rows a made table gives for a key that holds only some of its key columns.
pub(crate) const PARTIAL_KEY_ROWS: PlanEstimate = PlanEstimate(10 * 1_024);
/// The least and the greatest rows a made table is sized at from its input.
pub(crate) const MADE_ROWS: (PlanEstimate, PlanEstimate) =
    (PlanEstimate(512), PlanEstimate(2_000 * 1_024));
/// The ancestors of one element.
pub(crate) const ANCESTORS: PlanEstimate = PlanEstimate(8 * 1_024);
/// The descendants of one element.
pub(crate) const DESCENDANTS: PlanEstimate = PlanEstimate(30 * 1_024);
/// What a walk down edges that tests thin reaches from one element: 30 by 0.3 for one test.
pub(crate) const THINNED_DESCENDANTS: PlanEstimate = PlanEstimate(9 * 1_024);
/// The rows one step over a closure's edges gives, by which the walk is sized: up to the first (1.35) it climbs a chain;
/// under the second (a parent's children) its edges are thinned by tests; from there on it reaches what lies under an element.
pub(crate) const CLOSURE_STEP: (PlanEstimate, PlanEstimate) =
    (PlanEstimate(1_382), PlanEstimate(4 * 1_024));
/// The share of its rows a test of equality with a constant keeps: a tenth.
pub(crate) const CONSTANT_EQUAL_KEEPS: PlanEstimate = PlanEstimate(102);
/// The share an equality of two cells keeps: three tenths.
pub(crate) const EQUAL_KEEPS: PlanEstimate = PlanEstimate(307);
/// The share an inequality keeps: 0.95.
pub(crate) const UNEQUAL_KEEPS: PlanEstimate = PlanEstimate(973);
/// The share any other test keeps: seven tenths.
pub(crate) const TEST_KEEPS: PlanEstimate = PlanEstimate(717);
/// The steps of a well-keyed call: a use ranks as the rows it gives, or as its steps over this many when that is more.
pub(crate) const KEYED_STEPS: u64 = 20;
/// The least rank of a use of a definition: 1.05, just after a read of one row by its key.
pub(crate) const USE_RANK: PlanEstimate = PlanEstimate(1_075);
/// A decision goes before any access ranked above one row: a read by its key goes first.
pub(crate) const DECISION_WAITS: PlanEstimate = PlanEstimate(1_024);
/// A decision of this many steps or more is dear: it also waits for an access ranked up to 1.35.
pub(crate) const DEAR_DECISION: (PlanEstimate, PlanEstimate) =
    (PlanEstimate(10 * 1_024), PlanEstimate(1_382));
/// A decision of this many steps or more also waits for an access ranked up to 2.05.
pub(crate) const DEARER_DECISION: (PlanEstimate, PlanEstimate) =
    (PlanEstimate(20 * 1_024), PlanEstimate(2_099));
/// A use bound on a column of a definition that has no program for it reads the definition's table: each bound column is
/// taken to keep a tenth of the table's rows.
pub(crate) const TABLE_KEY_KEEPS: PlanEstimate = PlanEstimate(102);
