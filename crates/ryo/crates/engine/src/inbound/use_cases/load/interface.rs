//! What `load` promises: the selected rules' plans lowered in process into one program set, or the program set an earlier encode
//! wrote for the same pack digest and engine version.

use super::errors::LoadError;
use super::types::{LoadSource, LoadedRules};
use ryo_values::domain::descriptions::types::KindVocabulary;

/// Makes a rule set's programs.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::SelectedRules;
/// use ryo_engine::inbound::use_cases::load::interface::LoadUseCase;
/// use ryo_engine::domain::statistics::types::StatisticsBytes;
/// use ryo_engine::inbound::use_cases::load::types::LoadSource;
/// use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
/// use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
///
/// let kinds = KindVocabulary { language: LanguageName::default(), kinds: Vec::new(), fields: Vec::new(), fixed: Vec::new() };
/// assert!(LoadUseCaseImpl::new().load(LoadSource::Plans(SelectedRules(&[]), StatisticsBytes(&[])), &kinds).is_ok());
/// ```
pub trait LoadUseCase: Send + Sync {
    /// Lowers `Plans` as one pack, answering the programs of the rules it accepted and each rule it refused with the reason, or
    /// decodes `Pack`.
    ///
    /// # Errors
    ///
    /// `PackVersion`, `PackDigest` or `PackCorrupt` for a pack. A rule that cannot be loaded is not an error: it is in `refused`.
    fn load(
        &self,
        source: LoadSource<'_>,
        kinds: &KindVocabulary,
    ) -> Result<LoadedRules, LoadError>;
}
