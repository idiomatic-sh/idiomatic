//! The registered-case and native-observation documents as plain values: no transport encoding.

/// Named text and yes/no values of the receipt documents.
pub mod text;

/// One diagnostic as a document records it.
pub mod finding;

/// A documented difference from the reference tool.
pub mod reference_discrepancy;

/// One native side's observation row.
pub mod observation_document;
