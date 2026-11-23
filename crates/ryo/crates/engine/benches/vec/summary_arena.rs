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
        let mut offset = id / 64;
        let start = self.words.len();
        self.words
            .resize(start + 1 + (id / 64 - offset) as usize, 0);
        self.words[start + (id / 64 - offset) as usize] = 1u64 << (id % 64);
        let mut lowest = id / 64;
        for id in iter {
            let word = id / 64;
            if word < offset {
                let end = self.words.len();
                self.words.resize(end + offset as usize, 0);
                self.words.copy_within(start..end, start + offset as usize);
                self.words[start..start + offset as usize].fill(0);
                offset = 0;
            }
            lowest = lowest.min(word);
            let i = start + (word - offset) as usize;
            if i >= self.words.len() {
                self.words.resize(i + 1, 0);
            }
            self.words[i] |= 1u64 << (id % 64);
        }
        let gap = (lowest - offset) as usize;
        if gap != 0 {
            self.words.copy_within(start + gap.., start);
            self.words.truncate(self.words.len() - gap);
            offset = lowest;
        }
        Bits {
            start,
            first: offset as usize,
            len: self.words.len() - start,
        }
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
        let len = l.len();
        let start = self.words.len();
        self.words.resize(start + len, 0);
        let (inputs, output) = self.words.split_at_mut(start);
        for ((w, x), y) in output.iter_mut().zip(&inputs[l]).zip(&inputs[r]) {
            *w = x & y;
        }
        for mask in [a, b] {
            if mask.first == first && mask.len == len && output == &inputs[mask.range()] {
                self.words.truncate(start);
                return mask;
            }
        }
        let Some(lo) = output.iter().position(|&w| w != 0) else {
            self.words.truncate(start);
            return Bits::default();
        };
        let hi = output.iter().rposition(|&w| w != 0).unwrap() + 1;
        if lo != 0 {
            output.copy_within(lo..hi, 0);
        }
        self.words.truncate(start + hi - lo);
        Bits {
            start,
            first: first + lo,
            len: hi - lo,
        }
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
