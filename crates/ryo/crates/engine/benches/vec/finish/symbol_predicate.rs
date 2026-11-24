//! A bound text predicate shares answers for immutable symbols within its reader.
use super::{Run, Ty, V, text_of};
use regex::bytes::Regex;
use std::{cell::RefCell, fmt, rc::Rc};

pub(super) struct Bound<T> {
    pub(super) pattern: T,
    // Each cell is unseen, false, or true. Allocate a page only after a test in it.
    pages: RefCell<Vec<Option<Box<[u64; 16]>>>>,
}

impl<T> Bound<T> {
    pub(super) fn new(pattern: T) -> Self {
        Self { pattern, pages: RefCell::default() }
    }

    #[inline]
    fn answer(&self, id: u32, test: impl FnOnce() -> bool) -> bool {
        let id = id as usize;
        let page = id / 512;
        let mut pages = self.pages.borrow_mut();
        if pages.len() <= page {
            pages.resize_with(page + 1, || None);
        }
        let words = pages[page].get_or_insert_with(Default::default);
        let word = &mut words[id % 512 / 32];
        let shift = 2 * (id % 32);
        match (*word >> shift) & 3 {
            0 => {
                let yes = test();
                *word |= (1 + u64::from(yes)) << shift;
                yes
            }
            state => state == 2,
        }
    }
}

// Plan identities contain only the predicate, independent of previously visited rows.
impl<T: fmt::Debug> fmt::Debug for Bound<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.pattern.fmt(f) }
}

#[derive(Clone, Debug)]
pub(super) enum TextTest {
    Regex(Rc<Bound<Regex>>),
    Glob(Rc<PreparedGlob>),
    LowerEqual(Rc<Bound<Rc<[u8]>>>),
}

impl TextTest {
    #[inline]
    pub(super) fn symbol(&self, run: &Run, id: u32) -> bool {
        match self {
            Self::Glob(glob) => glob.symbol(run, id),
            Self::Regex(re) => re.answer(id, || re.pattern.is_match(run.text(id))),
            Self::LowerEqual(lit) => lit.answer(id, || {
                String::from_utf8_lossy(run.text(id)).to_lowercase().as_bytes() == lit.pattern.as_ref()
            }),
        }
    }

    pub(super) fn bytes(&self, bytes: &[u8]) -> bool {
        match self {
            Self::Glob(glob) => glob.bytes(bytes),
            Self::Regex(re) => re.pattern.is_match(bytes),
            Self::LowerEqual(lit) => String::from_utf8_lossy(bytes).to_lowercase().as_bytes() == lit.pattern.as_ref(),
        }
    }

    #[inline]
    pub(super) fn value(&self, run: &Run, v: V) -> bool {
        if let Self::Glob(glob) = self { return glob.matches(run, v); }
        if v.ty() == Ty::Sym { self.symbol(run, v.1 as u32) } else { self.bytes(&text_of(run, v)) }
    }
}

pub(super) fn folded_literal(run: &mut Run, a: &super::E, b: &super::E) -> Option<super::E> {
    use super::E;
    let (E::Call(name, args), E::Text(id)) = (a,b) else { return None };
    let [col @ E::Col(_, _, Ty::Sym)] = args.as_slice() else { return None };
    if name != "lower" { return None }
    let test = run.folded_literals.entry(*id)
        .or_insert_with(|| Rc::new(Bound::new(run.texts[*id as usize].clone())));
    Some(E::TextTest(Box::new(col.clone()), TextTest::LowerEqual(test.clone())))
}

/// Interpreted globs often see a file's path on every row of its relations.
pub(super) struct PreparedGlob {
    pattern: u32,
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
