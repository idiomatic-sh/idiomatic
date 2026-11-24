//! Shared file predicates. An active read evaluates only the input gates
//! needed to settle it. The result is reused by every decision in that file.
use rustc_hash::FxHashMap;
use std::cell::{Cell, RefCell};

#[derive(Default)]
pub(super) struct Circuit {
    pub(super) leaves: usize,
    gates: Vec<[u32; 2]>,
    // One byte per variable: zero is undecided, one is true, two is false.
    values: Vec<Cell<u8>>,
    pending: RefCell<Vec<usize>>,
}

pub(super) struct Compiler {
    pub(super) circuit: Circuit,
    pairs: FxHashMap<[u32; 2], u32>,
}

impl Compiler {
    pub(super) fn new(leaves: usize) -> Self {
        // Literal zero is true and literal one is false.
        let values = vec![Cell::new(0); 1 + leaves];
        values[0].set(1);
        Self { circuit: Circuit { leaves, values, ..Default::default() }, pairs: FxHashMap::default() }
    }

    pub(super) fn and(&mut self, a: u32, b: u32) -> u32 {
        if a == 0 || a == b { return b; }
        if b == 0 { return a; }
        if a == 1 || b == 1 || a == (b ^ 1) { return 1; }
        // A conjunction already proves either input. Its complement is true
        // whenever either input is false. Recognize both while building paths.
        for (outer, term) in [(a, b), (b, a)] {
            let gate = (outer as usize / 2).checked_sub(self.circuit.leaves + 1);
            let Some(inputs) = gate.map(|g| self.circuit.gates[g]) else { continue };
            if outer & 1 == 0 {
                if inputs.contains(&term) { return outer; }
                if inputs.contains(&(term ^ 1)) { return 1; }
            } else if inputs.contains(&(term ^ 1)) {
                return term;
            }
        }
        let pair = [a.min(b), a.max(b)];
        *self.pairs.entry(pair).or_insert_with(|| {
            let key = 2 * self.circuit.values.len() as u32;
            self.circuit.gates.push(pair);
            self.circuit.values.push(Cell::new(0));
            key
        })
    }
}

impl Circuit {
    /// Intermediate guards created while lowering the operator graph may have
    /// no final reader. Keep the gates reachable from actual decisions only.
    pub(super) fn retain_outputs(&mut self, roots: impl Iterator<Item = u32>) -> Vec<u32> {
        let first = self.leaves + 1;
        let total = first + self.gates.len();
        let mut live = vec![false; total];
        for root in roots { live[root as usize / 2] = true; }
        for (i, &[a, b]) in self.gates.iter().enumerate().rev() {
            if live[first + i] {
                live[a as usize / 2] = true;
                live[b as usize / 2] = true;
            }
        }
        let mut map = (0..first as u32).collect::<Vec<_>>();
        map.resize(total, 0);
        let mut kept = 0;
        for i in 0..self.gates.len() {
            if !live[first + i] { continue; }
            self.gates[kept] = self.gates[i].map(|v| Self::remap(&map, v));
            map[first + i] = (first + kept) as u32;
            kept += 1;
        }
        self.gates.truncate(kept);
        self.values.truncate(first + kept);
        map
    }

    pub(super) fn remap(map: &[u32], literal: u32) -> u32 {
        2 * map[literal as usize / 2] + (literal & 1)
    }

    pub(super) fn begin_file(&mut self) {
        if let Some(values) = self.values.get_mut(1..) { values.fill(Cell::new(0)); }
    }

    #[inline]
    fn cached(&self, literal: u32, read: &mut impl FnMut(usize) -> bool) -> Option<bool> {
        let i = (literal / 2) as usize;
        let mut raw = self.values[i].get();
        if raw == 0 {
            if i > self.leaves { return None; }
            raw = if read(i - 1) { 1 } else { 2 };
            self.values[i].set(raw);
        }
        Some((raw == 1) ^ (literal & 1 != 0))
    }

    #[inline]
    pub(super) fn permits(&self, literal: u32, mut read: impl FnMut(usize) -> bool) -> bool {
        literal == 0 || self.cached(literal, &mut read).unwrap_or_else(|| self.settle(literal, &mut read))
    }

    // Only an undecided AND needs an entry on the stack. File evidence is
    // immutable until the next file; every consulted input is read once.
    fn settle(&self, literal: u32, read: &mut impl FnMut(usize) -> bool) -> bool {
        let first = self.leaves + 1;
        let mut pending = self.pending.borrow_mut();
        pending.push((literal / 2) as usize);
        while let Some(&node) = pending.last() {
            let [a, b] = self.gates[node - first];
            let value = match self.cached(a, read) {
                None => { pending.push((a / 2) as usize); continue; }
                Some(false) => false,
                Some(true) => match self.cached(b, read) {
                    None => { pending.push((b / 2) as usize); continue; }
                    Some(value) => value,
                },
            };
            self.values[node].set(if value { 1 } else { 2 });
            pending.pop();
        }
        self.cached(literal, read).expect("the requested gate was settled")
    }

    pub(super) fn counts(&self) -> (usize, usize) { (self.leaves, self.gates.len()) }
}

#[cfg(test)]
mod tests {
    use super::Compiler;

    #[test]
    fn deep_decisions_share_a_file_and_reset_between_files() {
        let count = 4096;
        let mut compiler = Compiler::new(count);
        let gate = (1..=count as u32).fold(0, |root, i| compiler.and(root, 2 * i));
        let mut circuit = compiler.circuit;
        let map = circuit.retain_outputs([gate].into_iter());
        let gate = super::Circuit::remap(&map, gate);
        circuit.begin_file();
        let mut reads = 0;
        assert!(circuit.permits(gate, |_| { reads += 1; true }));
        assert_eq!(reads, count);
        assert!(!circuit.permits(gate ^ 1, |_| panic!("shared gate was settled")));
        circuit.begin_file();
        assert!(!circuit.permits(gate, |i| i != 0));
        assert!(circuit.permits(gate ^ 1, |_| panic!("negated gate shares its answer")));
    }

    #[test]
    fn unneeded_inputs_are_skipped_and_shared_inputs_are_read_once() {
        let mut compiler = Compiler::new(3);
        let ab = compiler.and(2, 4);
        let ac = compiler.and(2, 6);
        let mut circuit = compiler.circuit;
        circuit.begin_file();
        assert!(!circuit.permits(ab, |i| { assert_eq!(i, 0); false }));
        assert!(!circuit.permits(ac, |_| panic!("the first term already failed")));
        circuit.begin_file();
        let mut seen = Vec::new();
        assert!(circuit.permits(ac, |i| { seen.push(i); true }));
        assert_eq!(seen, [0, 2]);
    }

    #[test]
    fn shared_gates_keep_their_truth_across_files() {
        let mut compiler = Compiler::new(3);
        let (a, b, c) = (2, 4, 6);
        let either = compiler.and(b, c ^ 1) ^ 1;
        let discarded = compiler.and(a, b);
        assert_eq!(compiler.and(discarded, b), discarded);
        assert_eq!(compiler.and(discarded, b ^ 1), 1);
        assert_eq!(compiler.and(discarded ^ 1, b ^ 1), b ^ 1);
        let answer = compiler.and(a, either);
        assert_eq!(compiler.and(either, a), answer);
        assert_eq!(compiler.and(a, a ^ 1), 1);
        let mut circuit = compiler.circuit;
        assert_eq!(circuit.counts(), (3, 3));
        let map = circuit.retain_outputs([answer].into_iter());
        let answer = super::Circuit::remap(&map, answer);
        assert_eq!(circuit.counts(), (3, 2));
        for assignment in [0, 0, 7, 1, 2, 3, 4, 5, 6, 7, 7] {
            let bits = [assignment & 1 != 0, assignment & 2 != 0, assignment & 4 != 0];
            circuit.begin_file();
            assert_eq!(circuit.permits(answer, |i| bits[i]), bits[0] && (!bits[1] || bits[2]));
        }
    }
}
