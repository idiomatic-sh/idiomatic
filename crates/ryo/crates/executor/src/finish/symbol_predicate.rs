//! A bound text predicate shares answers for immutable symbols within its reader.
use super::{Run, Ty, V, text_of};
use regex::bytes::Regex;
use std::{cell::RefCell, fmt, rc::Rc};

pub(crate) struct Bound<T> {
    pub(crate) pattern: T,
    pub(crate) last_file: std::cell::Cell<Option<(u32, bool)>>,
    pub(crate) pages: RefCell<AnswerPages>,
}

#[derive(Default)]
pub(crate) struct AnswerPages {
    // Zero means this range of 512 symbols has never been queried.
    pub(crate) slots: Vec<u32>,
    pub(crate) tested: Vec<TestedPage>,
    pub(crate) exceptions: Vec<[u64; 8]>,
}

pub(crate) struct TestedPage {
    pub(crate) seen: [u64; 8],
    pub(crate) default: bool,
    // Only a page with both outcomes needs a second bitmap.
    pub(crate) exceptions: Option<std::num::NonZeroU32>,
}

impl<T> Bound<T> {
    pub fn new(pattern: T) -> Self {
        Self { pattern, last_file: Default::default(), pages: RefCell::default() }
    }

    #[inline]
    pub(crate) fn file_answer(&self, id: u32, test: impl FnOnce() -> bool) -> bool {
        if let Some((previous, answer)) = self.last_file.get() && previous == id {
            return answer;
        }
        let answer = test();
        self.last_file.set(Some((id, answer)));
        answer
    }

    #[inline]
    pub fn answer(&self, id: u32, test: impl FnOnce() -> bool) -> bool {
        let id = id as usize;
        let page = id / 512;
        let mut pages = self.pages.borrow_mut();
        if pages.slots.len() <= page {
            pages.slots.resize(page + 1, 0);
        }
        let word = id % 512 / 64;
        let bit = 1u64 << (id % 64);
        let slot = pages.slots[page];
        if slot == 0 {
            let answer = test();
            let mut seen = [0; 8];
            seen[word] = bit;
            pages.tested.push(TestedPage { seen, default: answer, exceptions: None });
            pages.slots[page] = pages.tested.len() as u32;
            return answer;
        }
        let slot = slot as usize - 1;
        let entry = &pages.tested[slot];
        if entry.seen[word] & bit != 0 {
            let opposite = entry.exceptions.is_some_and(|index| {
                pages.exceptions[index.get() as usize - 1][word] & bit != 0
            });
            return entry.default ^ opposite;
        }
        let answer = test();
        if answer != entry.default {
            let index = match entry.exceptions {
                Some(index) => index.get() as usize - 1,
                None => {
                    let index = pages.exceptions.len();
                    pages.exceptions.push([0; 8]);
                    pages.tested[slot].exceptions = std::num::NonZeroU32::new((index + 1) as u32);
                    index
                }
            };
            pages.exceptions[index][word] |= bit;
        }
        pages.tested[slot].seen[word] |= bit;
        answer
    }
}

// Plan identities contain only the predicate, independent of previously visited rows.
impl<T: fmt::Debug> fmt::Debug for Bound<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.pattern.fmt(f) }
}

#[derive(Clone, Debug)]
pub(crate) enum TextTest {
    Regex(Rc<Bound<Rc<Regex>>>),
    Glob(Rc<PreparedGlob>),
    Any(Rc<Bound<Box<[TextTest]>>>),
    LowerEqual(Rc<Bound<Rc<[u8]>>>),
}

impl TextTest {
    #[inline]
    pub(crate) fn symbol(&self, run: &Run, id: u32) -> bool {
        match self {
            Self::Glob(glob) => glob.symbol(run, id),
            Self::Any(tests) => tests.answer(id, || tests.pattern.iter().any(|test| test.symbol(run, id))),
            Self::Regex(re) => re.answer(id, || re.pattern.is_match(run.text(id))),
            Self::LowerEqual(lit) => lit.answer(id, || {
                String::from_utf8_lossy(run.text(id)).to_lowercase().as_bytes() == lit.pattern.as_ref()
            }),
        }
    }

    pub(crate) fn bytes(&self, bytes: &[u8]) -> bool {
        match self {
            Self::Glob(glob) => glob.bytes(bytes),
            Self::Any(tests) => tests.pattern.iter().any(|test| test.bytes(bytes)),
            Self::Regex(re) => re.pattern.is_match(bytes),
            Self::LowerEqual(lit) => String::from_utf8_lossy(bytes).to_lowercase().as_bytes() == lit.pattern.as_ref(),
        }
    }

    #[inline]
    pub fn value(&self, run: &Run, v: V) -> bool {
        if let Self::Glob(glob) = self { return glob.matches(run, v); }
        if v.ty() == Ty::File && let Self::Any(tests) = self { return tests.file_answer(v.1 as u32, || self.bytes(run.paths[v.1 as usize].as_bytes())); }
        if v.ty() == Ty::Sym { self.symbol(run, v.1 as u32) } else { self.bytes(&text_of(run, v)) }
    }
}

pub(crate) fn folded_literal(run: &mut Run, a: &super::E, b: &super::E) -> Option<super::E> {
    use super::E;
    let (E::Call(name, args), E::Text(id)) = (a,b) else { return None };
    let [col @ E::Col(_, _, Ty::Sym)] = args.as_slice() else { return None };
    if name != "lower" { return None }
    let test = run.folded_literals.entry(*id)
        .or_insert_with(|| Rc::new(Bound::new(run.texts[*id as usize].clone())));
    Some(E::TextTest(Box::new(col.clone()), TextTest::LowerEqual(test.clone())))
}

/// Interpreted globs often see a file's path on every row of its relations.
pub(crate) struct PreparedGlob {
    pub(crate) pattern: u32,
    pub(crate) matcher: Bound<ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs>,
}

// The compiled plan identity is independent of which subject was visited last.
impl fmt::Debug for PreparedGlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Glob").field(&self.pattern).finish()
    }
}

impl PreparedGlob {
    pub fn new(pattern: u32, matcher: ryo_matching::inbound::use_cases::matching::types::CompiledTextGlobs) -> Self {
        Self { pattern, matcher: Bound::new(matcher) }
    }

    pub(crate) fn bytes(&self, bytes: &[u8]) -> bool {
        use ryo_matching::inbound::use_cases::matching::{
            interface::MatchingUseCase, types::{MatchOutcome, SearchText}, use_case::MatchingUseCaseImpl,
        };
        let subject = String::from_utf8_lossy(bytes);
        MatchingUseCaseImpl::new().matches_text_globs(&self.matcher.pattern, SearchText(&subject)) == MatchOutcome::Matches
    }

    pub(crate) fn symbol(&self, run: &Run, id: u32) -> bool {
        self.matcher.answer(id, || self.bytes(run.text(id)))
    }

    pub(crate) fn matches(&self, run: &Run, value: V) -> bool {
        match value.ty() {
            Ty::Sym => self.symbol(run, value.1 as u32),
            Ty::File => self.matcher.file_answer(value.1 as u32, || self.bytes(run.paths[value.1 as usize].as_bytes())),
            _ => self.bytes(&text_of(run, value)),
        }
    }
}

/// One bit per listed symbol. Empty stretches of the dictionary occupy no space.
#[derive(Debug)]
pub(crate) struct SymbolSet(Box<[(u64, u64)]>);

impl SymbolSet {
    pub(crate) fn bind(run: &Run, mut words: Vec<u64>) -> Rc<Self> {
        words.sort_unstable();
        words.dedup();
        let mut sets = run.symbol_sets.borrow_mut();
        if let Some(set) = sets.get(words.as_slice()) {
            return Rc::clone(set);
        }
        let mut blocks = Vec::new();
        for group in words.chunk_by(|a, b| a / 64 == b / 64) {
            let block = group[0] / 64;
            let bits = group.iter().fold(0, |bits, word| bits | (1u64 << (word % 64)));
            blocks.push((block, bits));
        }
        let set = Rc::new(Self(blocks.into_boxed_slice()));
        sets.insert(words.into_boxed_slice(), Rc::clone(&set));
        set
    }

    #[inline]
    pub(crate) fn contains(&self, word: u64) -> bool {
        let block = word / 64;
        let bit = 1u64 << (word % 64);
        if let [(index, bits)] = self.0.as_ref() {
            return *index == block && bits & bit != 0;
        }
        self.0.binary_search_by_key(&block, |entry| entry.0)
            .is_ok_and(|i| self.0[i].1 & bit != 0)
    }
}
