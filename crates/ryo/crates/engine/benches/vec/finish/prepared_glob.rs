use super::{Run, Ty, V, text_of};
use std::{cell::RefCell, fmt};

/// Interpreted globs often see a file's path on every row of its relations.
pub(super) struct PreparedGlob {
    pub(super) pattern: u32,
    matcher: ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs,
    // Each run owns its spelling dictionary. Two bits store unknown, false, or true.
    pages: RefCell<Vec<Option<Box<[u64; 16]>>>>,
}

// The compiled plan identity is independent of the subjects already tested.
impl fmt::Debug for PreparedGlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Glob").field(&self.pattern).finish()
    }
}

impl PreparedGlob {
    pub(super) fn new(pattern: u32, matcher: ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs) -> Self {
        Self { pattern, matcher, pages: RefCell::default() }
    }

    fn bytes(&self, bytes: &[u8]) -> bool {
        use ryo_matching::inbound::use_cases::matching::{
            interface::MatchingUseCase, types::{MatchOutcome, SearchText}, use_case::MatchingUseCaseImpl,
        };
        let subject = String::from_utf8_lossy(bytes);
        MatchingUseCaseImpl::new().matches_text_globs(&self.matcher, SearchText(&subject)) == MatchOutcome::Matches
    }

    fn symbol(&self, run: &Run, id: u32) -> bool {
        let index = id as usize;
        let page = index / 512;
        let mut pages = self.pages.borrow_mut();
        if pages.len() <= page {
            pages.resize_with(page + 1, || None);
        }
        let words = pages[page].get_or_insert_with(Default::default);
        let word = &mut words[index % 512 / 32];
        let shift = 2 * (index % 32);
        match (*word >> shift) & 3 {
            0 => {
                let answer = self.bytes(run.text(id));
                *word |= (1 + u64::from(answer)) << shift;
                answer
            }
            state => state == 2,
        }
    }

    pub(super) fn matches(&self, run: &Run, value: V) -> bool {
        match value.ty() {
            Ty::Sym => self.symbol(run, value.1 as u32),
            Ty::File => self.symbol(run, run.path_sym[value.1 as usize]),
            _ => self.bytes(&text_of(run, value)),
        }
    }
}
