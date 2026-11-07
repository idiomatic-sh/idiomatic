//! The row of every entry, routed to the group that states it.
use super::*;

impl Entry {
    pub(in super::super) fn row(self) -> Row {
        match self {
            Entry::AdmitView | Entry::AdmitPartition | Entry::AcquireStored | Entry::AcquireStoredWithRoute | Entry::TryAcquireRows | Entry::ScopedBuild | Entry::PrepareGraph | Entry::AcquireGraph | Entry::AcquireGraphDomain | Entry::GraphRow | Entry::DemandAnswers | Entry::Reach | Entry::Axis | Entry::Nearest | Entry::PrepareProgramFile => acquiring::row(self),
            Entry::ViewRead | Entry::ContractRead | Entry::ConstructionRead | Entry::DependencyRead | Entry::DependencyBackings | Entry::GraphRead | Entry::OrientationRead | Entry::MembersRead | Entry::ValidateOwner | Entry::RootValues => reading::row(self),
            Entry::Counts | Entry::ForPredicate | Entry::MetadataBytes | Entry::OwnerNew | Entry::AccessPrepare | Entry::OwnedBytes | Entry::PeakRequiredViews | Entry::PeakWorkingBytes | Entry::ReleasedEntries | Entry::RequiredBackendOverlap | Entry::RetainedBytes | Entry::WorkingBytes | Entry::Report | Entry::SlotReplace | Entry::SlotRead | Entry::OptionalLease | Entry::RequiredLease | Entry::Clear | Entry::Pin | Entry::Release | Entry::Wanted => exempted::row(self),
        }
    }
}

mod acquiring;
mod exempted;
mod reading;
