//! The rows of the entries that report or retire, and that no view crosses.
use super::super::*;

pub(super) fn row(entry: Entry) -> Row {
    let graph = owner!("graph.rs");
    let setup = owner!("setup.rs");
    let view = owner!("acquisition/admission/service.rs");
    let owner = owner!("mod.rs");
    let accounting = "accounting: bytes and counts the owner reports, no view crosses";
    match entry {
        Entry::Counts => exempt(owner, "counts", accounting),
        Entry::ForPredicate => exempt(
            owner,
            "for_predicate",
            "a context key built from a prepared plan; no view crosses",
        ),
        Entry::MetadataBytes => exempt(owner, "metadata_bytes", accounting),
        Entry::OwnerNew => exempt(
            owner,
            "new",
            "constructs the owner; the relation owner holds it",
        ),
        Entry::AccessPrepare => exempt(
            owner!("types.rs"),
            "prepare",
            "request-time metadata through the owner-issued capability; the builder behind it is owner-private (AUTHORITY_ACCESS, SEALED_PORT)",
        ),
        Entry::OwnedBytes => exempt(owner, "owned_bytes", accounting),
        Entry::PeakRequiredViews => exempt(owner, "peak_required_views", accounting),
        Entry::PeakWorkingBytes => exempt(owner, "peak_working_bytes", accounting),
        Entry::ReleasedEntries => exempt(owner, "released_entries", accounting),
        Entry::RequiredBackendOverlap => exempt(owner, "required_backend_overlap", accounting),
        Entry::RetainedBytes => exempt(owner, "retained_bytes", accounting),
        Entry::WorkingBytes => exempt(owner, "working_bytes", accounting),
        Entry::Report => exempt(
            owner!("diagnostics.rs"),
            "report",
            "diagnostic report of counts",
        ),
        Entry::SlotReplace => exempt(
            owner!("graphs/service.rs"),
            "replace",
            "slot helper of the owner's own graph tables",
        ),
        Entry::SlotRead => exempt(
            owner!("graphs/service.rs"),
            "slot",
            "slot helper of the owner's own graph tables",
        ),
        Entry::OptionalLease => exempt(
            owner!("leases.rs"),
            "optional_lease",
            "ledger lease issued to publications, not a view",
        ),
        Entry::RequiredLease => exempt(
            owner!("leases.rs"),
            "required_lease",
            "ledger lease issued to publications, not a view",
        ),
        Entry::Clear => exempt(
            owner!("lifetime.rs"),
            "clear",
            "generation retirement; releases every view",
        ),
        Entry::Pin => exempt(
            owner!("lifetime.rs"),
            "pin",
            "population pin; the lifecycle controls release it",
        ),
        Entry::Release => exempt(
            owner!("lifetime.rs"),
            "release",
            "population release; the lifecycle controls drive it",
        ),
        Entry::SelectorWork => exempt(owner!("observations.rs"), "selector_work", accounting),
        Entry::Wanted => exempt(
            setup,
            "wanted",
            "request-side relation list; no view crosses",
        ),
        _ => unreachable!(),
    }
}
