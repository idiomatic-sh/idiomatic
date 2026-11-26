//! A projected column has its source's physical u64 representation (including its null)
//! and the output schema carries its logical type. Dedup compares these words exactly.
use super::super::{Schema, Table};
use std::rc::Rc;

pub struct Projection {
    pub source: usize,
    pub schema: Rc<Schema>,
    pub picks: Box<[usize]>,
    pub ident: bool,
    pub distinct: bool,
}

impl Projection {
    pub fn kind(&self) -> &'static str {
        if self.ident {
            "identity"
        } else if self.distinct {
            "distinct"
        } else {
            "copy"
        }
    }

    pub fn gather(&self, input: &Table, selection: Option<&[usize]>) -> Table {
        let n = selection.map_or(input.n, <[_]>::len);
        let mut out = Table::with_rows(self.schema.names.clone(), self.schema.tys.clone(), n);
        for (&pick, dest) in self.picks.iter().zip(&mut out.cols) {
            let col = &input.cols[pick];
            match selection {
                None => dest.extend_from_slice(col),
                Some(rows) => dest.extend(rows.iter().map(|&r| col[r])),
            }
        }
        out.n = n;
        out
    }
}

/// Scratch is used after the input is ready, so a recursive input read cannot alias it.
#[derive(Default)]
pub struct Work {
    pub hashes: Vec<u64>,
    pub slots: Vec<usize>,
    pub kept: Vec<usize>,
}

impl Work {
    /// Return true if a selection is needed. Kept rows are always in first-occurrence order.
    pub fn distinct(&mut self, input: &Table, picks: &[usize]) -> bool {
        let n = input.n;
        let Some((&first, rest)) = picks.split_first() else {
            self.kept.clear();
            self.kept.push(0);
            return true;
        };
        let col = &input.cols[first];
        // A strictly increasing column proves uniqueness of the entire projected row.
        if col.windows(2).all(|w| w[0] < w[1]) {
            return false;
        }
        let hashes = if rest.is_empty() {
            col.as_slice()
        } else {
            self.hashes.clear();
            self.hashes.extend_from_slice(col);
            for &c in rest {
                for (hash, &word) in self.hashes.iter_mut().zip(&input.cols[c]) {
                    *hash = hash.rotate_left(13).wrapping_mul(0x517c_c1b7_2722_0a95) ^ word;
                }
            }
            &self.hashes
        };
        let size = (2 * n).next_power_of_two();
        self.slots.clear();
        self.slots.resize(size, usize::MAX);
        self.kept.clear();
        let shift = u64::BITS - size.trailing_zeros();
        for (r, &hash) in hashes.iter().enumerate() {
            let mut pos = (hash.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> shift) as usize;
            loop {
                let earlier = self.slots[pos];
                if earlier == usize::MAX {
                    self.slots[pos] = r;
                    // Materialize row numbers only after the first duplicate.
                    if !self.kept.is_empty() {
                        self.kept.push(r);
                    }
                    break;
                }
                if hashes[earlier] == hash
                    && (rest.is_empty()
                        || picks
                            .iter()
                            .all(|&c| input.cols[c][earlier] == input.cols[c][r]))
                {
                    if self.kept.is_empty() {
                        self.kept.extend(0..r);
                    }
                    break;
                }
                pos = (pos + 1) & (size - 1);
            }
        }
        !self.kept.is_empty()
    }
}
