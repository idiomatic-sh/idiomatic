//! Immutable file-domain bitsets share one allocation. Handles remain valid
//! when more words are appended. Clearing a file retains storage for the next.
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct Bits {
    start: usize,
    len: usize,
}
impl Bits {
    pub(super) fn is_empty(self) -> bool {
        self.len == 0
    }
    pub(super) fn len(self) -> usize {
        self.len
    }
    pub(super) fn range(self) -> std::ops::Range<usize> {
        self.start..self.start + self.len
    }
}
#[derive(Default)]
pub(super) struct Arena {
    pub(super) words: Vec<u64>,
    pub(super) work: Vec<u64>,
}
impl Arena {
    #[inline]
    pub(super) fn word(&self, b: Bits, i: usize) -> u64 {
        self.words[b.start + i]
    }
    pub(super) fn commit(&mut self, data: &[u64]) -> Bits {
        let len = data.iter().rposition(|w| *w != 0).map_or(0, |i| i + 1);
        let start = self.words.len();
        self.words.extend_from_slice(&data[..len]);
        Bits { start, len }
    }
    pub(super) fn finish(&mut self) -> Bits {
        let start = self.words.len();
        self.words.extend_from_slice(&self.work);
        let len = self.work.len();
        self.work.clear();
        Bits { start, len }
    }
    pub(super) fn bits(&mut self, ids: impl IntoIterator<Item = u32>) -> Bits {
        for id in ids {
            super::insert(&mut self.work, id);
        }
        self.finish()
    }
    pub(super) fn intersects(&self, a: Bits, b: Bits) -> bool {
        self.words[a.range()]
            .iter()
            .zip(&self.words[b.range()])
            .any(|(l, r)| l & r != 0)
    }
    pub(super) fn intersect(&mut self, a: Bits, b: Bits) -> Bits {
        if a.is_empty() || b.is_empty() {
            return Bits::default();
        }
        if a == b {
            return a;
        }
        self.work.extend(
            self.words[a.range()]
                .iter()
                .zip(&self.words[b.range()])
                .map(|(l, r)| l & r),
        );
        while self.work.last() == Some(&0) {
            self.work.pop();
        }
        self.finish()
    }
    pub(super) fn union(&mut self, a: Bits, b: Bits) -> Bits {
        let (a, b) = if a.len >= b.len { (a, b) } else { (b, a) };
        if b.is_empty() || a == b {
            return a;
        }
        self.work.extend(
            self.words[a.range()]
                .iter()
                .zip(&self.words[b.range()])
                .map(|(l, r)| l | r),
        );
        self.work
            .extend_from_slice(&self.words[a.start + b.len..a.start + a.len]);
        self.finish()
    }
}
