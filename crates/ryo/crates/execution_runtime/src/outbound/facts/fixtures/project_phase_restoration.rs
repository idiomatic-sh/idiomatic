//! The facts fakes the execution use case's `project_phase_restoration` boundary scenarios inject.
use super::project_phase_source_facts::CapturedFacts;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsFinish;
use crate::outbound::facts::types::KeptBucket;
use ryo_values::domain::stored_facts::types::Bucket;

/// The restored producer frontier and, in its own bucket, the consumer's file frontier.
pub struct ConsumerFrontier<'a> {
    pub producer: &'a CapturedFacts,
    pub consumer: &'a CapturedFacts,
}

impl FactsFinish for ConsumerFrontier<'_> {
    fn kept(&self, bucket: Bucket) -> Result<KeptBucket, FactsError> {
        if bucket == Bucket(10) {
            return self.consumer.kept(bucket);
        }
        self.producer.kept(bucket)
    }
}
