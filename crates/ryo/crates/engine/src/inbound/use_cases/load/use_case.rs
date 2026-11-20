//! The `load` use case: plans through the statistics, the lowering, the passes and the encoding; a pack to the decoder.

use super::errors::LoadError;
use super::interface::LoadUseCase;
use super::service::sequenced;
use super::types::{LoadSource, LoadedRules};
use crate::domain::lowering::service::lowered;
use crate::domain::packs::errors::PacksError;
use crate::domain::packs::service::decoded as unpacked;
use crate::domain::statistics::service::decoded;
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
    fn load(
        &self,
        source: LoadSource<'_>,
        kinds: &KindVocabulary,
    ) -> Result<LoadedRules, LoadError> {
        match source {
            LoadSource::Plans(rules, statistics) => {
                // Statistics that do not read are no statistics: the rules load by the lowering's own order.
                let statistics = decoded(statistics, kinds).unwrap_or_default();
                Ok(sequenced(rules, &statistics, &|selected| {
                    lowered(selected, &statistics, kinds)
                }))
            }
            LoadSource::Pack(bytes, digest) => unpacked(bytes, digest)
                .map(|programs| LoadedRules {
                    programs,
                    refused: Vec::new(),
                })
                .map_err(|error| match error {
                    PacksError::Version => LoadError::PackVersion,
                    PacksError::Digest => LoadError::PackDigest,
                    PacksError::Corrupt => LoadError::PackCorrupt,
                }),
        }
    }
}
