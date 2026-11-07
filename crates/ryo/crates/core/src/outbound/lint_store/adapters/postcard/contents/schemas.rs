//! The contents document of the postcard store: its wire shapes and their conversion from and into the core's tables.

use super::super::types::BucketSpan;
use super::super::types::FileOffset;
use super::super::types::SectionLength;
use super::super::types::SectionSpan;
use super::super::types::TableOfContents;
use std::borrow::Cow;
use ryo_values::domain::stored_facts::types::Bucket;

/// Where a section is in the file, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct SpanSchema {
    /// Where it starts.
    pub start: u64,
    /// How long it is.
    pub length: u64,
}

/// Where the words of one bucket are in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct BucketSpanSchema {
    /// The bucket.
    pub bucket: u32,
    /// Where its section is.
    pub span: SpanSchema,
}

/// The table of contents, last in the file: which build wrote it and where each section is.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct ContentsSchema<'a> {
    /// The build.
    #[serde(borrow)]
    pub build: Cow<'a, str>,
    /// The small tables.
    pub tables: SpanSchema,
    /// What the files handed but their words.
    pub rest: SpanSchema,
    /// The words of each bucket, in the order of the buckets.
    pub buckets: Vec<BucketSpanSchema>,
}

impl From<SpanSchema> for SectionSpan {
    fn from(schema: SpanSchema) -> Self {
        Self {
            start: FileOffset(schema.start),
            length: SectionLength(schema.length),
        }
    }
}

impl From<SectionSpan> for SpanSchema {
    fn from(span: SectionSpan) -> Self {
        Self {
            start: span.start.0,
            length: span.length.0,
        }
    }
}

impl From<BucketSpanSchema> for BucketSpan {
    fn from(schema: BucketSpanSchema) -> Self {
        Self {
            bucket: Bucket(schema.bucket),
            span: SectionSpan::from(schema.span),
        }
    }
}

impl From<BucketSpan> for BucketSpanSchema {
    fn from(span: BucketSpan) -> Self {
        Self {
            bucket: span.bucket.0,
            span: SpanSchema::from(span.span),
        }
    }
}

/// The table of contents `build` writes, as its bytes.
pub(in crate::outbound::lint_store::adapters::postcard) fn contents_bytes(
    build: &str,
    contents: &TableOfContents,
) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_stdvec(&ContentsSchema {
        build: Cow::Borrowed(build),
        tables: SpanSchema::from(contents.tables),
        rest: SpanSchema::from(contents.rest),
        buckets: contents
            .buckets
            .iter()
            .map(|each| BucketSpanSchema::from(*each))
            .collect(),
    })
}

/// The table of contents `bytes` hold, when `build` wrote them.
pub(in crate::outbound::lint_store::adapters::postcard) fn contents(build: &str, bytes: &[u8]) -> Option<TableOfContents> {
    // A table of contents that does not read back is a store kept by another build or cut short: nothing is kept.
    let Ok(contents) = postcard::from_bytes::<ContentsSchema<'_>>(bytes) else {
        return None;
    };
    (*contents.build == *build).then(|| TableOfContents {
        tables: SectionSpan::from(contents.tables),
        rest: SectionSpan::from(contents.rest),
        buckets: contents.buckets.into_iter().map(BucketSpan::from).collect(),
    })
}
