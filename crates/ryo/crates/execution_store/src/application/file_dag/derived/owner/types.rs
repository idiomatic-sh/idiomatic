//! Counters, orders and answers of the derived owner, each named for what it counts.
use crate::application::file_dag::derived::ledger::types::ByteCount;

/// The order in which an entry was admitted; a smaller order was admitted first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AdmissionOrder(pub usize);

/// Live pins on one population; zero means the population is unpinned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PinCount(pub usize);

/// Entries whose reverse dependency lists were walked by a release.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ReleasedEntries(pub u64);

/// Physical builds, including admission refusals that built invocation-locally.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuildCount(pub u64);

/// Acquisitions answered by an existing entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct HitCount(pub u64);

/// Acquisitions whose optional retention was refused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RefusalCount(pub u64);

/// Successful reuse of a retained immutable index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ReuseCount(pub u64);

/// What the owner built, answered from an entry and refused, in that order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BuildCounts(pub BuildCount, pub HitCount, pub RefusalCount);

/// Whether an admission found room for its cost under the file's bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Room {
    Fits,
    Full,
}

/// Whether an admission is also charged against the smaller memo sub-budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemoBudget {
    Charged,
    Exempt,
}

/// Whether a retained payload is pinned by a live required view or consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PinState {
    Pinned,
    Free,
}

/// What reclaiming optional payload could free, and whether pinned payload blocked more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReclaimOutlook {
    pub available: Option<crate::application::file_dag::derived::ledger::types::Credit>,
    pub pinned: PinState,
}

/// Required views live at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ViewCount(pub usize);

/// The exact dense ordering of a membership view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MembershipOrder(pub usize);

/// How many of a key's population slots are in use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct PopulationCount(pub u8);

/// A scope handed to one finite raw order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScopeId(pub usize);

/// The most entries the owner retains.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EntryLimit(pub usize);

/// Whether the file's demanded requirements have been prepared, or the owner serves the project phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FilePreparation {
    #[default]
    Unprepared,
    Prepared,
    /// The owner of the project phase: it reads every file's frontier rows, is never prepared for one file and holds no
    /// graph cache, so an axis or a reach it runs walks its tables.
    Project,
}

/// Whether two immutable identities describe the same retained thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IdentityCompatibility {
    Compatible,
    Incompatible,
}

impl From<bool> for Room {
    fn from(fits: bool) -> Self {
        if fits { Self::Fits } else { Self::Full }
    }
}

impl From<bool> for IdentityCompatibility {
    fn from(same: bool) -> Self {
        if same {
            Self::Compatible
        } else {
            Self::Incompatible
        }
    }
}

impl PartialEq<u64> for BuildCount {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u64> for BuildCount {
    fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialEq<u64> for HitCount {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u64> for HitCount {
    fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialEq<u64> for RefusalCount {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialEq<u64> for ReleasedEntries {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl std::ops::AddAssign<u64> for BuildCount {
    fn add_assign(&mut self, other: u64) {
        self.0 += other;
    }
}

impl std::ops::AddAssign<u64> for HitCount {
    fn add_assign(&mut self, other: u64) {
        self.0 += other;
    }
}

impl std::ops::AddAssign<u64> for RefusalCount {
    fn add_assign(&mut self, other: u64) {
        self.0 += other;
    }
}

impl std::ops::AddAssign<u64> for ReleasedEntries {
    fn add_assign(&mut self, other: u64) {
        self.0 += other;
    }
}

impl std::ops::AddAssign<u64> for ReuseCount {
    fn add_assign(&mut self, other: u64) {
        self.0 += other;
    }
}

impl std::ops::AddAssign<usize> for ViewCount {
    fn add_assign(&mut self, other: usize) {
        self.0 += other;
    }
}

impl std::ops::AddAssign<usize> for PinCount {
    fn add_assign(&mut self, other: usize) {
        self.0 += other;
    }
}

impl PartialEq<usize> for ByteCount {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<usize> for ByteCount {
    fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}
