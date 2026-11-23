//! File-local immutable word windows. Many token and subtree sets start late
//! in a file, so their handles include the first represented word number.
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct Bits {
    start: usize,
    first: usize,
    len: usize,
}
impl Bits {
    pub(super) fn is_empty(self) -> bool {
        self.len == 0
    }
    pub(super) fn len(self) -> usize {
        self.first + self.len
    }
    pub(super) fn first(self) -> usize {
        self.first
    }
    pub(super) fn range(self) -> std::ops::Range<usize> {
        self.start..self.start + self.len
    }
}
#[derive(Default)]
pub(super) struct Arena {
    pub(super) census: Option<Census>,
    pub(super) window: bool,
    pub(super) words: Vec<u64>,
    pub(super) work: Vec<u64>,
}
impl Arena {
    pub(super) fn absolute(&self, b: Bits) -> Vec<u64> {
        let mut out = vec![0; b.first()];
        out.extend_from_slice(&self.words[b.range()]);
        out
    }
    pub(super) fn word(&self, b: Bits, i: usize) -> u64 {
        i.checked_sub(b.first)
            .filter(|&d| d < b.len)
            .map_or(0, |d| self.words[b.start + d])
    }
    fn add(&mut self, first: usize, data: &[u64]) -> Bits {
        let Some(lo) = data.iter().position(|&x| x != 0) else {
            return Bits::default();
        };
        let hi = data.iter().rposition(|&x| x != 0).unwrap() + 1;
        if let Some(m) = &mut self.census {
            let trimmed = &data[lo..hi];
            m.values += 1;
            m.one_word += u64::from(trimmed.len() == 1);
            let mut hits = 0;
            for &w in trimmed {
                m.nonzero_words += u64::from(w != 0);
                hits += u64::from(w.count_ones());
            }
            m.nodes += hits;
            m.singletons += u64::from(hits == 1);
        }
        let lo = if self.window { lo } else { 0 };
        let b = Bits {
            start: self.words.len(),
            first: first + lo,
            len: hi - lo,
        };
        self.words.extend_from_slice(&data[lo..hi]);
        b
    }
    pub(super) fn commit(&mut self, data: &[u64]) -> Bits {
        self.add(0, data)
    }
    fn flush(&mut self, first: usize) -> Bits {
        let mut data = std::mem::take(&mut self.work);
        let b = self.add(first, &data);
        data.clear();
        self.work = data;
        b
    }
    pub(super) fn finish(&mut self) -> Bits {
        self.flush(0)
    }
    pub(super) fn bits(&mut self, ids: impl IntoIterator<Item = u32>) -> Bits {
        let mut iter = ids.into_iter();
        let Some(id) = iter.next() else {
            return Bits::default();
        };
        let mut offset = if self.window { id / 64 } else { 0 };
        super::insert(&mut self.work, id - offset * 64);
        for id in iter {
            if id / 64 < offset {
                // An unordered group can cross its starting word. Switch to
                // absolute indices once; future IDs require no shifts.
                let moved = self.work.len();
                let gap = offset as usize;
                self.work.resize(moved + gap, 0);
                self.work.copy_within(0..moved, gap);
                self.work[..gap].fill(0);
                offset = 0;
            }
            super::insert(&mut self.work, id - offset * 64);
        }
        self.flush(offset as usize)
    }
    fn overlap(
        a: Bits,
        b: Bits,
    ) -> Option<(usize, std::ops::Range<usize>, std::ops::Range<usize>)> {
        let first = a.first.max(b.first);
        let end = a.len().min(b.len());
        if first >= end {
            return None;
        }
        let (l, r) = (a.start + first - a.first, b.start + first - b.first);
        Some((first, l..l + (end - first), r..r + (end - first)))
    }
    pub(super) fn intersects(&self, a: Bits, b: Bits) -> bool {
        Self::overlap(a, b).is_some_and(|(_, l, r)| {
            self.words[l]
                .iter()
                .zip(&self.words[r])
                .any(|(l, r)| l & r != 0)
        })
    }
    pub(super) fn intersect(&mut self, a: Bits, b: Bits) -> Bits {
        if a == b {
            return a;
        }
        let Some((first, l, r)) = Self::overlap(a, b) else {
            return Bits::default();
        };
        self.work
            .extend(self.words[l].iter().zip(&self.words[r]).map(|(l, r)| l & r));
        self.flush(first)
    }
    pub(super) fn union(&mut self, a: Bits, b: Bits) -> Bits {
        if a.is_empty() || a == b {
            return b;
        }
        if b.is_empty() {
            return a;
        }
        let first = a.first.min(b.first);
        let end = a.len().max(b.len());
        self.work.resize(end - first, 0);
        self.work[a.first - first..a.len() - first].copy_from_slice(&self.words[a.range()]);
        for (l, &r) in self.work[b.first - first..b.len() - first]
            .iter_mut()
            .zip(&self.words[b.range()])
        {
            *l |= r;
        }
        self.flush(first)
    }
}
/// Add every bit in a half-open interval without visiting its members.
pub(super) fn cover(words: &mut Vec<u64>, range: std::ops::Range<u32>) {
    if range.is_empty() {
        return;
    }
    let a = (range.start / 64) as usize;
    let b = ((range.end - 1) / 64) as usize;
    if words.len() <= b {
        words.resize(b + 1, 0);
    }
    let lo = u64::MAX << (range.start % 64);
    let hi = u64::MAX >> (63 - ((range.end - 1) % 64));
    if a == b {
        words[a] |= lo & hi;
    } else {
        words[a] |= lo;
        words[a + 1..b].fill(u64::MAX);
        words[b] |= hi;
    }
}

impl Arena {
    fn group_bounds(sets: &[Bits], and: bool) -> std::ops::Range<usize> {
        let (mut from, mut end) = if and {
            (0, usize::MAX)
        } else {
            (usize::MAX, 0)
        };
        for &s in sets {
            if and {
                from = from.max(s.first());
                end = end.min(s.len());
            } else {
                from = from.min(s.first());
                end = end.max(s.len());
            }
        }
        from..end
    }
    pub(super) fn common(&self, sets: &[Bits]) -> bool {
        Self::group_bounds(sets, true).any(|i| {
            let mut w = u64::MAX;
            for &s in sets {
                w &= self.word(s, i);
                if w == 0 {
                    return false;
                }
            }
            true
        })
    }
    pub(super) fn group(&mut self, sets: &[Bits], and: bool) -> Bits {
        if let [v] = sets {
            return *v;
        }
        let range = Self::group_bounds(sets, and);
        let first = range.start;
        for i in range {
            let mut w = if and { u64::MAX } else { 0 };
            for &s in sets {
                if and {
                    w &= self.word(s, i);
                    if w == 0 {
                        break;
                    }
                } else {
                    w |= self.word(s, i);
                }
            }
            self.work.push(w);
        }
        self.flush(first)
    }
}

impl Arena {
    pub(super) fn identical(&self, a: Option<Bits>, b: Option<Bits>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a == b || a.first() == b.first() && self.words[a.range()] == self.words[b.range()]
            }
            _ => false,
        }
    }
}

#[derive(Default)]
pub(super) struct Census {
    values: u64,
    one_word: u64,
    singletons: u64,
    nonzero_words: u64,
    nodes: u64,
}
impl Census {
    pub(super) fn report(&self) {
        println!(
            "materialised masks {} one-word {} singleton {} nonzero words {} selected node occurrences {}",
            self.values, self.one_word, self.singletons, self.nonzero_words, self.nodes
        );
    }
}
