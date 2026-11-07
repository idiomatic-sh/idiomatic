//! Collector wire bytes are distinct from captured child output.
/// One complete JSON protocol message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectorMessage(pub Vec<u8>);
