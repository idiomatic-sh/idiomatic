//! Catalog columns requested by a consumer. Empty projections retain row counts.

/// The catalog positions a consumer reads from a relation. Row count is independent of this selection.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColumnProjection {
    /// Every column, including columns beyond the first machine word.
    All,
    /// Only the selected columns numbered 0 through 63.
    First(u64),
    /// Selected columns in catalog order, one bit per column.
    Words(Vec<u64>),
}

impl Default for ColumnProjection {
    fn default() -> Self { Self::First(0) }
}

impl ColumnProjection {
    /// The selected bits for 64 consecutive catalog columns.
    #[inline]
    pub fn word(&self, index: usize) -> u64 {
        match self {
            Self::All => u64::MAX,
            Self::First(mask) => if index == 0 { *mask } else { 0 },
            Self::Words(words) => words.get(index).copied().unwrap_or(0),
        }
    }

    /// Whether a catalog column is selected.
    #[inline]
    pub fn contains(&self, column: usize) -> bool {
        self.word(column / 64) & (1 << (column % 64)) != 0
    }

    /// Select one further catalog column.
    pub fn insert(&mut self, column: usize) {
        let index = column / 64;
        let bit = 1 << (column % 64);
        match self {
            Self::All => return,
            Self::First(mask) if index == 0 => { *mask |= bit; return; }
            Self::First(mask) => *self = Self::Words(vec![*mask]),
            Self::Words(_) => {}
        }
        if let Self::Words(words) = self {
            if words.len() <= index { words.resize(index + 1, 0); }
            words[index] |= bit;
        }
    }

    /// Select every column of either projection.
    pub fn extend(&mut self, other: &Self) {
        match (self, other) {
            (Self::All, _) => {}
            (dest, Self::All) => *dest = Self::All,
            (Self::First(a), Self::First(b)) => *a |= b,
            (dest @ Self::First(_), Self::Words(b)) => {
                let mut words = b.clone();
                if words.is_empty() { return; }
                words[0] |= dest.word(0);
                *dest = Self::Words(words);
            }
            (Self::Words(a), Self::First(b)) => {
                if a.is_empty() { a.push(*b); } else { a[0] |= b; }
            }
            (Self::Words(a), Self::Words(b)) => {
                if a.len() < b.len() { a.resize(b.len(), 0); }
                for (x, y) in a.iter_mut().zip(b) { *x |= y; }
            }
        }
    }

    /// Whether every column in the other projection is selected here.
    pub fn covers(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::All, _) => true,
            (_, Self::All) => false,
            (_, Self::First(mask)) => self.word(0) & mask == *mask,
            (_, Self::Words(words)) => words.iter().enumerate().all(|(i, mask)| self.word(i) & mask == *mask),
        }
    }

    /// The number of selected columns, or the maximum for a whole-schema request.
    pub fn count(&self) -> usize {
        match self {
            Self::All => usize::MAX,
            Self::First(mask) => mask.count_ones() as usize,
            Self::Words(words) => words.iter().map(|w| w.count_ones() as usize).sum(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ColumnProjection;

    #[test]
    fn combines_file_and_operator_demands_beyond_one_word() {
        let mut result = ColumnProjection::default();
        let mut extra = ColumnProjection::default();
        for c in [2, 63, 127] { result.insert(c); }
        for c in [64, 128] { extra.insert(c); }
        assert!(!result.covers(&extra));
        result.extend(&extra);
        assert!(result.covers(&extra));
        assert_eq!(result.count(), 5);
        for c in 0..193 {
            assert_eq!(result.contains(c), [2, 63, 64, 127, 128].contains(&c));
        }
        assert!(!ColumnProjection::First(u64::MAX).contains(64));
        assert!(ColumnProjection::All.covers(&result));
        assert!(!result.covers(&ColumnProjection::All));
    }
}
