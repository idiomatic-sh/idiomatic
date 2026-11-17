//! What each node of a plan did over the files one worker ran while counting was on: how many times it was made and how many
//! of those wrote no row, the rows its kernels read and wrote, and the wall time spent in its own kernels (its sources' time
//! not included), all of it and that of the makings that wrote no row.

use crate::domain::physical_plan::types::{NodeFed, PlanNodeId};
use ryo_values::domain::rows::types::RowCount;
use std::time::{Duration, Instant};

/// How many times a node was made.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::node_counters::value::Makings;
/// assert_eq!(Makings::default(), Makings(0));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Makings(pub u64);

/// What one node did over the files counted.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::node_counters::value::{Makings, NodeWork};
/// let work = NodeWork::default();
/// assert_eq!((work.runs, work.empty_runs), (Makings(0), Makings(0)));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeWork {
    /// How many times it was made: once in each file that made it.
    pub runs: Makings,
    /// How many of those makings wrote no row.
    pub empty_runs: Makings,
    /// The rows its kernels read, every input summed: what it was fed.
    pub rows_in: RowCount,
    /// The rows it wrote: its held rows, or the rows a streamed stage handed its reader.
    pub rows_out: RowCount,
    /// The wall time spent making it, the time spent making the nodes it read not included.
    pub time: Duration,
    /// The part of `time` spent in the makings that wrote no row.
    pub empty_time: Duration,
}

/// A node made at least once, and what it did.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::PlanNodeId;
/// use ryo_operators::domain::physical_plan::types::node_counters::value::{CountedNode, NodeWork};
/// let counted = CountedNode { node: PlanNodeId(0), work: NodeWork::default() };
/// assert_eq!(counted.node, PlanNodeId(0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CountedNode {
    /// The node.
    pub node: PlanNodeId,
    /// What it did.
    pub work: NodeWork,
}

/// One making of a node: the node, the rows it was fed, every input summed, and the rows it wrote. The run keeps its worst, the
/// one that wrote the most rows per row fed of those that wrote at least the run budget's floor, whether counting is on or
/// off.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::PlanNodeId;
/// use ryo_operators::domain::physical_plan::types::node_counters::value::WorstMaking;
/// use ryo_values::domain::rows::types::RowCount;
/// let making = WorstMaking { node: PlanNodeId(3), fed: RowCount(2), wrote: RowCount(4) };
/// assert_eq!(making.wrote, RowCount(4));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorstMaking {
    /// The node made.
    pub node: PlanNodeId,
    /// The rows it was fed, every input summed.
    pub fed: RowCount,
    /// The rows it wrote.
    pub wrote: RowCount,
}

kinds! {
    /// The kind of a node's kernels, as the counters report it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::node_counters::value::NodeKind;
    /// assert_ne!(NodeKind::Filter, NodeKind::Probe);
    /// ```
    pub enum NodeKind {
        /// A filter.
        Filter,
        /// A lookup of a shared index.
        Lookup,
        /// A projection.
        Project,
        /// An extension.
        Extend,
        /// A probe: a join, an antijoin or an intersection.
        Probe,
        /// The rows of one input not in the other, or both inputs' rows without repeats.
        Distinct,
        /// A group.
        Group,
        /// A reach over edges.
        Reach,
        /// A reach from seed rows.
        SeededReach,
        /// An axis walk.
        Axis,
        /// An ordered walk.
        OrderedWalk,
        /// A worklist.
        Worklist,
        /// The components of a graph.
        Components,
        /// A map of each row's text.
        TextMap,
        /// A band join.
        BandJoin,
        /// An ordered match.
        OrderedMatch,
        /// A rendering.
        Render,
        /// A scan of rows the engine holds.
        ScanBound,
    }
}

/// The counts of every node of a plan one worker made while counting was on, by node id.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::node_counters::value::NodeCounters;
/// let mut counters = NodeCounters::default();
/// counters.merge(NodeCounters::default());
/// assert_eq!(counters.counts().count(), 0);
/// ```
#[derive(Clone, Debug, Default)]
pub struct NodeCounters {
    /// Each node's work, by node id.
    pub(crate) nodes: Vec<NodeWork>,
    /// Each node's making in progress, by node id: what is known of it before both its rows and its time are counted.
    pub(crate) open: Vec<OpenMaking>,
    /// The time the timed spans nested in the open one took, which the open one does not count as its own.
    pub(crate) nested: Duration,
}

/// What is known of a node's making before both its rows and its time are counted: a streamed stage's time comes before its
/// rows are counted, a held node's rows before the time of the span that made it ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum OpenMaking {
    /// Nothing is counted of it yet, or all of it is.
    #[default]
    Unknown,
    /// Its time so far, its rows not counted yet.
    Timed(Duration),
    /// Its rows are counted: whether it wrote none; the time of the span that made it is still to come.
    Counted(Output),
}

/// Whether a making wrote rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Output {
    /// It wrote none.
    Empty,
    /// It wrote some.
    Rows,
}

/// Where a node was made: in the timed span that makes it, still open, or as a stage of a chain, whose time is all counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MadeIn {
    /// In the span that makes it, which counts its time when it closes.
    OpenSpan,
    /// As a stage of a chain: its time was counted as its rows were pushed.
    Chain,
}

/// A timed span that is open: when it started, and the nested time of the span it is nested in.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OpenSpan {
    /// When it started.
    started: Instant,
    /// The nested time of the span it is nested in, so far.
    outer: Duration,
}

impl NodeCounters {
    /// Adds `other`'s counts to these, node by node: another worker's counts of the same plan.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::node_counters::value::NodeCounters;
    /// let mut counters = NodeCounters::default();
    /// counters.merge(NodeCounters::default());
    /// assert_eq!(counters.counts().count(), 0);
    /// ```
    pub fn merge(&mut self, other: Self) {
        if self.nodes.len() < other.nodes.len() {
            self.nodes.resize(other.nodes.len(), NodeWork::default());
        }
        for (work, more) in self.nodes.iter_mut().zip(other.nodes) {
            work.add(&more);
        }
    }

    /// Each node that was made at least once, in node order, with what it did.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::node_counters::value::NodeCounters;
    /// assert!(NodeCounters::default().counts().next().is_none());
    /// ```
    pub fn counts(&self) -> impl Iterator<Item = CountedNode> + '_ {
        (0..)
            .map(PlanNodeId)
            .zip(self.nodes.iter().copied())
            .filter(|(_, work)| work.runs.0 > 0)
            .map(|(node, work)| CountedNode { node, work })
    }

    /// `node` was made once, `made_in` a span still open or as a chain's stage: it was fed `fed` and wrote `out` rows. The time
    /// it took before is counted to a making that wrote no row when it wrote none, and so is the time of the open span that
    /// made it, when it closes.
    pub(crate) fn made(&mut self, node: PlanNodeId, made_in: MadeIn, fed: NodeFed, out: RowCount) {
        let output = if out.0 == 0 {
            Output::Empty
        } else {
            Output::Rows
        };
        let after = match made_in {
            MadeIn::OpenSpan => OpenMaking::Counted(output),
            MadeIn::Chain => OpenMaking::Unknown,
        };
        let before = self
            .open_of(node)
            .map(|open| std::mem::replace(open, after));
        let Some(work) = self.work_of(node) else {
            return;
        };
        let fed = crate::domain::growth::service::fed_rows(&fed.0).0;
        work.runs.0 = work.runs.0.saturating_add(1);
        work.rows_in.0 = work.rows_in.0.saturating_add(fed);
        work.rows_out.0 = work.rows_out.0.saturating_add(out.0);
        if output == Output::Empty {
            work.empty_runs.0 = work.empty_runs.0.saturating_add(1);
            if let Some(OpenMaking::Timed(took)) = before {
                work.empty_time = work.empty_time.saturating_add(took);
            }
        }
    }

    /// Opens a timed span, nested in the one open now.
    pub(crate) fn opened(&mut self) -> OpenSpan {
        OpenSpan {
            started: Instant::now(),
            outer: std::mem::take(&mut self.nested),
        }
    }

    /// Closes `span`, the last one opened, and counts its time to `node`, the time of the spans nested in it taken out: to the
    /// making whose rows were counted in it, or else to the one whose rows are still to be counted.
    pub(crate) fn closed(&mut self, span: OpenSpan, node: PlanNodeId) {
        let took = span.started.elapsed();
        let nested = std::mem::replace(&mut self.nested, span.outer.saturating_add(took));
        let own = took.saturating_sub(nested);
        let output = self.open_of(node).and_then(|open| match *open {
            OpenMaking::Counted(output) => {
                *open = OpenMaking::Unknown;
                Some(output)
            }
            OpenMaking::Timed(before) => {
                *open = OpenMaking::Timed(before.saturating_add(own));
                None
            }
            OpenMaking::Unknown => {
                *open = OpenMaking::Timed(own);
                None
            }
        });
        if let Some(work) = self.work_of(node) {
            work.time = work.time.saturating_add(own);
            if output == Some(Output::Empty) {
                work.empty_time = work.empty_time.saturating_add(own);
            }
        }
    }

    /// `node`'s work, the counts grown to it.
    fn work_of(&mut self, node: PlanNodeId) -> Option<&mut NodeWork> {
        let at = usize::try_from(node.0).unwrap_or(usize::MAX);
        if self.nodes.len() <= at && at < usize::MAX {
            self.nodes.resize(at.saturating_add(1), NodeWork::default());
        }
        self.nodes.get_mut(at)
    }

    /// `node`'s making in progress, the makings grown to it.
    fn open_of(&mut self, node: PlanNodeId) -> Option<&mut OpenMaking> {
        let at = usize::try_from(node.0).unwrap_or(usize::MAX);
        if self.open.len() <= at && at < usize::MAX {
            self.open.resize(at.saturating_add(1), OpenMaking::Unknown);
        }
        self.open.get_mut(at)
    }
}

impl NodeWork {
    /// Adds `more`'s counts and times to these: the same node's on another worker, or another node's for a run's total.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_operators::domain::physical_plan::types::node_counters::value::{Makings, NodeWork};
    /// let mut work = NodeWork { runs: Makings(1), ..NodeWork::default() };
    /// work.add(&NodeWork { runs: Makings(2), ..NodeWork::default() });
    /// assert_eq!(work.runs, Makings(3));
    /// ```
    pub fn add(&mut self, more: &Self) {
        self.runs.0 = self.runs.0.saturating_add(more.runs.0);
        self.empty_runs.0 = self.empty_runs.0.saturating_add(more.empty_runs.0);
        self.rows_in.0 = self.rows_in.0.saturating_add(more.rows_in.0);
        self.rows_out.0 = self.rows_out.0.saturating_add(more.rows_out.0);
        self.time = self.time.saturating_add(more.time);
        self.empty_time = self.empty_time.saturating_add(more.empty_time);
    }
}
