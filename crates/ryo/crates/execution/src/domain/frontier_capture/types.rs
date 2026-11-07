//! The frontier a rule hands over: its packed words and the scope that bounds the capture.
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Words;

pub struct CaptureScope<'a> {
    pub file: ryo_relations::domain::relations::types::FileId,
    /// Current rule identity validated by the store before warm replay.
    pub rule_digest: ryo_values::domain::stored_facts::types::Digest,
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    pub retained: crate::domain::finish::types::Retained<'a>,
    pub words: &'a mut Words,
}
pub struct FrontierCapture {
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    pub outcome: Result<CapturedFrontier, OperatingError>,
}
pub struct CapturedFrontier {
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    pub bucket: ryo_values::domain::stored_facts::types::Bucket,
    /// Source-bound root and physical coverage retained with the packed words.
    pub proof: ryo_values::domain::records::types::FrontierProof,
    pub words: ryo_values::domain::stored_facts::types::PackedWords,
}
