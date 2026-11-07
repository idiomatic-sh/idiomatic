//! Plain data the documents of the ryo linter adapter exchange with its adapter.

/// The encoded worker request, ready to be written to its request file.
#[derive(Clone, Debug)]
pub(crate) struct RequestJson(pub String);

/// The bytes a child printed as its report.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ReportBytes<'a>(pub &'a [u8]);
