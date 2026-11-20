//! The `load` use case: plans to the lowering, a pack to the decoder.

use super::errors::LoadError;
use super::interface::LoadUseCase;
use super::types::LoadSource;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::service::lowered;
use crate::domain::packs::errors::PacksError;
use crate::domain::packs::service::decoded;
use crate::domain::programs::types::Programs;
use ryo_values::domain::descriptions::types::KindVocabulary;

/// Makes rule sets' programs in process.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
///
/// let _use_case = LoadUseCaseImpl::new();
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct LoadUseCaseImpl;

impl LoadUseCaseImpl {
    /// The use case; it holds no port.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
    ///
    /// let _use_case = LoadUseCaseImpl::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl LoadUseCase for LoadUseCaseImpl {
    fn load(&self, source: LoadSource<'_>, kinds: &KindVocabulary) -> Result<Programs, LoadError> {
        match source {
            LoadSource::Plans(rules) => lowered(rules, kinds).map_err(|error| match error {
                LoweringError::Unlowerable { rule, shape } => {
                    LoadError::Unlowerable { rule, shape }
                }
                LoweringError::Budget { rule } => LoadError::Budget { rule },
            }),
            LoadSource::Pack(bytes, digest) => {
                decoded(bytes, digest).map_err(|error| match error {
                    PacksError::Version => LoadError::PackVersion,
                    PacksError::Digest => LoadError::PackDigest,
                    PacksError::Corrupt => LoadError::PackCorrupt,
                })
            }
        }
    }
}
