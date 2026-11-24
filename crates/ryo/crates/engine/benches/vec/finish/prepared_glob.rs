use super::{Run, Ty, V, text_of};
use std::fmt;

/// Interpreted globs often see a file's path on every row of its relations.
pub(super) struct PreparedGlob {
    pub(super) pattern: u32,
    matcher: ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs,
    last: std::cell::Cell<Option<(u32, bool)>>,
}

// The compiled plan identity is independent of which subject was visited last.
impl fmt::Debug for PreparedGlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Glob").field(&self.pattern).finish()
    }
}

impl PreparedGlob {
    pub(super) fn new(pattern: u32, matcher: ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs) -> Self {
        Self { pattern, matcher, last: Default::default() }
    }

    fn bytes(&self, bytes: &[u8]) -> bool {
        use ryo_matching::inbound::use_cases::matching::{
            interface::MatchingUseCase, types::{MatchOutcome, SearchText}, use_case::MatchingUseCaseImpl,
        };
        let subject = String::from_utf8_lossy(bytes);
        MatchingUseCaseImpl::new().matches_text_globs(&self.matcher, SearchText(&subject)) == MatchOutcome::Matches
    }

    fn symbol(&self, run: &Run, id: u32) -> bool {
        if let Some((last, answer)) = self.last.get() && id == last {
            return answer;
        }
        let answer = self.bytes(run.text(id));
        self.last.set(Some((id, answer)));
        answer
    }

    pub(super) fn matches(&self, run: &Run, value: V) -> bool {
        match value.ty() {
            Ty::Sym => self.symbol(run, value.1 as u32),
            Ty::File => self.symbol(run, run.path_sym[value.1 as usize]),
            _ => self.bytes(&text_of(run, value)),
        }
    }
}
