//! The rows of the entries that read a leased view or handle: the accessor is nameable from a consumer.
use super::super::*;

pub(super) fn row(entry: Entry) -> Row {
    let graph = owner!("graph.rs");
    let view = owner!("acquisition/admission/service.rs");
    let handles = owner!("graphs/types/handles/service.rs");
    let owner = owner!("mod.rs");
    match entry {
        Entry::ViewRead => reads(
            view,
            "view",
            Authority::AcquiredView,
            "AcquiredView",
            ADMISSION,
        ),
        Entry::ContractRead => reads(
            view,
            "contract",
            Authority::AcquiredView,
            "AcquiredView",
            ADMISSION,
        ),
        Entry::GraphRead => reads(
            handles,
            "graph",
            Authority::GraphHandle,
            "GraphHandle",
            HANDLES,
        ),
        Entry::OrientationRead => reads(
            handles,
            "orientation",
            Authority::GraphHandle,
            "GraphHandle",
            HANDLES,
        ),
        Entry::MembersRead => reads(
            handles,
            "members",
            Authority::DomainHandle,
            "DomainHandle",
            HANDLES,
        ),
        Entry::ValidateOwner => reads(
            handles,
            "validate_owner",
            Authority::GraphHandle,
            "GraphHandle",
            HANDLES,
        ),
        Entry::RootValues => reads(
            owner!("graphs/types/handles/service.rs"),
            "values",
            Authority::RootAnswer,
            "RootAnswer",
            HANDLES,
        ),
        _ => unreachable!(),
    }
}
