//! The `stored_facts` service: free functions named by verb over what a run keeps of the words and the text of a file, with no IO and no ports.

/// The bucket the rows of the frontier's input at `input` of a rule wait in, the rule's first bucket being `first`.
#[must_use]
pub fn frontier_bucket(
    first: super::types::Bucket,
    input: crate::domain::plan_parts::types::InputNumber,
) -> super::types::Bucket {
    super::types::Bucket(
        first
            .0
            .saturating_add(u32::try_from(input.0).unwrap_or(u32::MAX)),
    )
}
