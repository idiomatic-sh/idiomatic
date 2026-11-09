//! One raw invocation record kept beside a captured process, as a plain value.
use crate::domain::documents::types::text::value::{
    CanonicalJson, ExitStatus, PathText, Pin, ReasonText, Sha256Text, Signal, YesNo,
};
use std::collections::BTreeMap;

/// One argument of the captured command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgumentText(pub String);

/// The name of one captured environment override.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EnvironmentName(pub String);

/// The value of one captured environment override.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentValue(pub String);

/// The normalized settings a capture is bound to, with the three parts raw metadata re-derives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedSettings {
    /// The normalized command line.
    pub arguments: Vec<ArgumentText>,
    /// The normalized environment overrides.
    pub environment: BTreeMap<EnvironmentName, EnvironmentValue>,
    /// The normalized working directory.
    pub directory: PathText,
    /// The whole settings in canonical sorted-key form.
    pub canonical: CanonicalJson,
}

/// A capture whose process identity was recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteInvocation {
    /// The directory the request ran under.
    pub actual_request_root: PathText,
    /// The sources the request read.
    pub files: Vec<PathText>,
    /// The executable that ran.
    pub executable_sha256: Sha256Text,
    /// The normalized settings digest.
    pub settings_sha256: Sha256Text,
    /// The normalized settings.
    pub normalized_settings: NormalizedSettings,
    /// The pinned profile.
    pub profile_sha256: Sha256Text,
    /// The tool's name.
    pub tool: ReasonText,
    /// The tool's version.
    pub version: ReasonText,
    /// The state boundary of the request.
    pub state: PathText,
    /// The raw command line.
    pub arguments: Vec<ArgumentText>,
    /// The raw environment overrides.
    pub environment: BTreeMap<EnvironmentName, EnvironmentValue>,
    /// The raw working directory.
    pub directory: PathText,
    /// The numeric exit status.
    pub exit: ExitStatus,
    /// The terminating signal.
    pub signal: Signal,
    /// Whether the observer used no deadline.
    pub observer_no_deadline: YesNo,
    /// The case sources.
    pub source_sha256: Pin,
    /// The declared file metadata.
    pub file_metadata_sha256: Pin,
}

/// A capture whose process identity could not be recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailedInvocation {
    /// The directory the request ran under.
    pub actual_request_root: PathText,
    /// The sources the request read.
    pub files: Vec<PathText>,
    /// The raw command line.
    pub arguments: Vec<ArgumentText>,
    /// The raw environment overrides.
    pub environment: BTreeMap<EnvironmentName, EnvironmentValue>,
    /// The raw working directory.
    pub directory: PathText,
    /// The numeric exit status.
    pub exit: ExitStatus,
    /// The terminating signal.
    pub signal: Signal,
    /// Whether the observer used no deadline.
    pub observer_no_deadline: YesNo,
    /// Why the identity could not be recorded.
    pub metadata_error: ReasonText,
    /// The case sources.
    pub source_sha256: Pin,
    /// The declared file metadata.
    pub file_metadata_sha256: Pin,
}

/// One raw invocation record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvocationDocument {
    /// The process identity was recorded.
    Complete(CompleteInvocation),
    /// The process identity could not be recorded.
    Failed(FailedInvocation),
}
