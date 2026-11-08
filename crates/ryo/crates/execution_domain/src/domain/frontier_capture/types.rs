//! The frontier a rule hands over: its packed words and the scope that bounds the capture.
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Words;

/// Where a captured frontier is kept: the file and worker it is captured in, what is retained of it and the words it is packed into.
pub struct CaptureScope<'a> {
    /// The file the frontier is captured in.
    pub file: ryo_relations::domain::relations::types::FileId,
    /// Current rule identity validated by the store before warm replay.
    pub rule_digest: ryo_values::domain::stored_facts::types::Digest,
    /// The worker capturing it.
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    /// What is retained of the file's rows.
    pub retained: crate::domain::finish::types::Retained<'a>,
    /// The words the frontier is packed into.
    pub words: &'a mut Words,
}
/// The outcome of capturing one frontier input.
pub struct FrontierCapture {
    /// The frontier input.
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    /// The captured frontier, or why it could not be captured.
    pub outcome: Result<CapturedFrontier, OperatingError>,
}
/// A captured frontier: its input, bucket, proof and packed words.
pub struct CapturedFrontier {
    /// The frontier input.
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    /// The bucket the frontier is stored in.
    pub bucket: ryo_values::domain::stored_facts::types::Bucket,
    /// Source-bound root and physical coverage retained with the packed words.
    pub proof: ryo_values::domain::records::types::FrontierProof,
    /// The frontier's rows, packed.
    pub words: ryo_values::domain::stored_facts::types::PackedWords,
}
