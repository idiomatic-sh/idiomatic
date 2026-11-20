//! What `load` promises: the selected rules' plans lowered in process into one program set, or the program set an earlier encode
//! wrote for the same pack digest and engine version.

use super::errors::LoadError;
use super::types::LoadSource;
use crate::domain::programs::types::Programs;
use ryo_values::domain::descriptions::types::KindVocabulary;

/// Makes a rule set's programs.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::lowering::types::SelectedRules;
/// use ryo_engine::inbound::use_cases::load::interface::LoadUseCase;
/// use ryo_engine::inbound::use_cases::load::types::LoadSource;
/// use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
/// use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
///
/// let kinds = KindVocabulary { language: LanguageName::default(), kinds: Vec::new() };
/// assert!(LoadUseCaseImpl::new().load(LoadSource::Plans(SelectedRules(&[])), &kinds).is_ok());
/// ```
pub trait LoadUseCase: Send + Sync {
    /// Lowers `Plans` (shared prefixes merged across rules, constants resolved to the run's words) or decodes `Pack`.
    ///
    /// # Errors
    ///
    /// `Unlowerable` or `Budget` naming the first rule that cannot run; `PackVersion`, `PackDigest` or `PackCorrupt` for a pack.
    fn load(&self, source: LoadSource<'_>, kinds: &KindVocabulary) -> Result<Programs, LoadError>;
}
