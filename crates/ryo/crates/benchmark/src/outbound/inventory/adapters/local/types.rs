//! Plain data the documents of the local inventory adapter exchange with its adapter.
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;

/// The bytes of a registration document, as read from its file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegistrationBytes<'a>(pub &'a CapturedBytes);

/// The bytes of one raw invocation record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InvocationBytes<'a>(pub &'a CapturedBytes);

/// The text of one case of a registered rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CaseText<'a>(pub &'a str);

/// The text of one documented difference of a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DiscrepancyText<'a>(pub &'a str);

/// An encoded public summary, ready to be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SummaryJson(pub CapturedBytes);

/// An encoded result row, ready to be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RowJson(pub CapturedBytes);

/// An encoded audit answer, ready to be printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuditJson(pub CapturedBytes);
