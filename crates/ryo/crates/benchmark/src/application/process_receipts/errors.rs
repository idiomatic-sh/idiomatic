//! Why a reference process receipt is refused.
use crate::domain::process_failure::errors::ProcessFailureError;
use crate::domain::registered_inputs::types::RefusalText;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;

/// A refused reference process receipt: each refusal of the registered and captured documents is its own variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ProcessReceiptsError {
    /// Refuses: the case registers no process expectation.
    #[error("invalid process registration: missing")]
    ProcessRegistrationMissing,
    /// Refuses: the reference is not the explicit process variant with its exact termination.
    #[error("reference process variant or exact termination identity differs")]
    ReferenceVariantDiffers,
    /// Refuses: the certificate's file metadata identity differs.
    #[error("process certificate file metadata identity differs")]
    FileMetadataIdentityDiffers,
    /// Refuses: the receipt names no request root.
    #[error("process receipt request root is missing")]
    RequestRootMissing,
    /// Refuses: the observation ran under another root than the receipt.
    #[error("process observation root differs from receipt")]
    ObservationRootDiffers,
    /// Refuses: the observation points at no raw process artifacts.
    #[error("missing raw process path")]
    MissingRawProcessPath,
    /// Refuses: a raw artifact belongs to another case identity.
    #[error("raw process artifact belongs to another case identity")]
    ArtifactBelongsToAnotherCase,
    /// Refuses: the captured bytes are absent.
    #[error("missing captured process bytes")]
    MissingCapturedProcessBytes,
    /// Refuses: the captured bytes do not hash to their pointer.
    #[error("captured process byte digest differs")]
    CapturedByteDigestDiffers,
    /// Refuses: the captured bytes differ from the registered templates.
    #[error("captured process bytes differ from registered templates")]
    CapturedBytesDifferFromTemplates,
    /// Refuses: the raw invocation record was not decoded.
    #[error("missing raw invocation")]
    MissingRawInvocation,
    /// Refuses: the capture failed to record its process identity.
    #[error("process metadata capture failed")]
    MetadataCaptureFailed,
    /// Refuses: the raw invocation differs in file metadata identity.
    #[error("raw process invocation differs at file_metadata_sha256")]
    InvocationFileMetadataDiffers,
    /// Refuses: the raw invocation differs in tool.
    #[error("raw process invocation differs at tool")]
    InvocationToolDiffers,
    /// Refuses: the raw invocation differs in version.
    #[error("raw process invocation differs at version")]
    InvocationVersionDiffers,
    /// Refuses: the raw invocation differs in request root.
    #[error("raw process invocation differs at actual_request_root")]
    InvocationRootDiffers,
    /// Refuses: the raw invocation differs in files.
    #[error("raw process invocation differs at files")]
    InvocationFilesDiffer,
    /// Refuses: the raw invocation differs in source identity.
    #[error("raw process invocation differs at source_sha256")]
    InvocationSourceDiffers,
    /// Refuses: the raw invocation differs in executable identity.
    #[error("raw process invocation differs at executable_sha256")]
    InvocationExecutableDiffers,
    /// Refuses: the raw invocation differs in settings identity.
    #[error("raw process invocation differs at settings_sha256")]
    InvocationSettingsDiffer,
    /// Refuses: the raw invocation differs in profile identity.
    #[error("raw process invocation differs at profile_sha256")]
    InvocationProfileDiffers,
    /// Refuses: the raw invocation differs in exit status.
    #[error("raw process invocation differs at exit")]
    InvocationExitDiffers,
    /// Refuses: the raw invocation records a signal.
    #[error("raw process invocation differs at signal")]
    InvocationSignalDiffers,
    /// Refuses: the raw invocation differs in observer deadline.
    #[error("raw process invocation differs at observer_no_deadline")]
    InvocationObserverDiffers,
    /// Refuses: the normalized settings do not hash to the pinned identity.
    #[error("missing pinned normalized invocation")]
    MissingPinnedNormalizedInvocation,
    /// Refuses: the raw invocation names no state boundary.
    #[error("raw invocation state is missing")]
    RawStateMissing,
    /// Refuses: raw argv, environment and directory do not establish the pinned settings.
    #[error("raw argv/environment do not establish pinned settings")]
    RawSettingsNotEstablished,
    /// Refuses: the process failure archive is not reproducible.
    #[error(transparent)]
    Archive(#[from] ProcessFailureError),
    /// A library or a neighbour refused with these words.
    #[error("{}", .0.0)]
    Refused(RefusalText),
}

impl From<String> for ProcessReceiptsError {
    fn from(text: String) -> Self {
        Self::Refused(RefusalText(text))
    }
}

impl From<ReportsError> for ProcessReceiptsError {
    fn from(error: ReportsError) -> Self {
        Self::Refused(RefusalText(error.to_string()))
    }
}

impl From<ProcessReceiptsError> for String {
    fn from(error: ProcessReceiptsError) -> Self {
        error.to_string()
    }
}
