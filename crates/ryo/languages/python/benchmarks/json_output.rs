//! JSON bytes supplied to a reference-tool test pipe or written as a benchmark report.
pub(crate) fn bytes<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(value)
}
