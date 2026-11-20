//! The frontier a rule hands over: its packed words and the scope that bounds the capture.
use crate::domain::operating::errors::OperatingError;

/// The outcome of capturing one frontier input.
#[derive(Debug)]
pub struct FrontierCapture {
    /// The frontier input.
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    /// The captured frontier, or why it could not be captured.
    pub outcome: Result<CapturedFrontier, OperatingError>,
}
/// A captured frontier: its input, bucket, proof and packed words.
#[derive(Debug)]
pub struct CapturedFrontier {
    /// The frontier input.
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    /// The bucket the frontier is stored in.
    pub bucket: ryo_values::domain::stored_facts::types::Bucket,
    /// Source-bound root and physical coverage retained with the packed words.
    pub proof: ryo_values::domain::records::types::FrontierProof,
    /// The frontier's rows, packed.
    pub words: ryo_values::domain::stored_facts::types::PackedWords,
    /// How many rows the frontier holds: none for a file that hands the input nothing.
    pub rows: ryo_values::domain::rows::types::RowCount,
    /// Spike (ryo-fmx3, host3): the frame holds no row and is complete: its words are the bucket's one clean frame, kept as
    /// a clean file of the bucket, not as a frame of its own.
    pub clean: bool,
}
