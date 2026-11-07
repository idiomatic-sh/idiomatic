//! The admitted population an observation hands over.
/// Opaque application handover of the actual admitted population. Packed bytes
/// may accompany it, but never substitute for its owner/generation evidence.
#[derive(Clone)]
pub struct ObservedPublication {
    pub(in crate::application::file_dag::program) admitted: crate::application::file_dag::program::admission::AdmittedInput,
}
