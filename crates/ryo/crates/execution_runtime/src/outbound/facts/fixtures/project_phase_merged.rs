//! The facts fakes the execution use case's `project_phase_merged` boundary scenarios inject.
use super::project_phase_source_facts::CapturedFacts;
use ryo_values::domain::stored_facts::types::Bucket;

/// The kept frontier of each of a rule's inputs, each in its own bucket.
pub struct Buckets(pub Vec<CapturedFacts>);

impl crate::outbound::facts::interface::FactsFinish for Buckets {
    fn kept(
        &self,
        bucket: Bucket,
    ) -> Result<crate::outbound::facts::types::KeptBucket, crate::outbound::facts::errors::FactsError>
    {
        self.0[usize::try_from(bucket.0).expect("a bucket number fits in usize")].kept(bucket)
    }
}
