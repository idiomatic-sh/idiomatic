//! Builds the receipts adapters, one function each, by constructor injection: a factory takes
//! the clients and settings the edge opened and returns the adapter built with them, so no
//! port method takes configuration. A factory builds nothing else, chooses nothing, and is
//! called only by an edge.

/// The local filesystem receipts adapter, appending where the edge chose.
#[must_use]
pub fn local(
    case_receipts: super::types::CaseReceipts,
) -> impl crate::outbound::receipts::interface::ReceiptsPort {
    super::adapters::local::adapter::ReceiptsLocalAdapter::new(case_receipts)
}
