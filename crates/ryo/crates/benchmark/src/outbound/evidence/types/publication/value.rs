//! The actual callable write boundary requires this workflow-issued capsule.
use ryo_benchmark_contracts::domain::measurements::types::PublicationSample;
/// Immutable borrowed observations whose original samples passed the application gate.
///
/// A neutral raw payload is insufficient to call the real BMF writer.
/// ```compile_fail,E0308
/// use ryo_benchmark::outbound::evidence::{factory,interface::EvidencePort};
/// use ryo_benchmark_contracts::domain::measurements::types::PublicationSample;
/// fn bypass(raw:&[PublicationSample<'_>]) {
///     factory::bmf(Box::new(Vec::<u8>::new())).publish(raw).unwrap();
/// }
/// ```
/// External code cannot turn its own raw flags into workflow-issued permission.
/// ```compile_fail,E0624
/// use ryo_benchmark::outbound::evidence::types::publication::value::Publication;
/// let forged=Publication::checked(&[]);
/// ```
/// An issued permission cannot be retargeted to different observations.
/// ```compile_fail,E0616
/// use ryo_benchmark::outbound::evidence::types::publication::value::Publication;
/// fn replace(permission:&mut Publication<'_>) { permission.samples=&[]; }
/// ```
#[derive(Debug)]
pub struct Publication<'a> {
    samples: &'a [PublicationSample<'a>],
}
impl<'a> Publication<'a> {
    /// The application calls this only after validating original issued samples.
    pub(crate) fn checked(samples: &'a [PublicationSample<'a>]) -> Self {
        Self { samples }
    }
    /// A destination may inspect or forward only this exact immutable payload.
    #[must_use]
    pub fn samples(&self) -> &[PublicationSample<'a>] {
        self.samples
    }
}
