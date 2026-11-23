//! File predicates have fixed dependencies. Compile their conjunctions and
//! disjunctions once, sharing equal pairs. Literals are adjacent truth and
//! complement bytes, so each gate needs two indexed reads and one AND.
use rustc_hash::FxHashMap;

#[derive(Default)]
pub(super) struct Circuit {
    pub(super) leaves: usize,
    gates: Vec<[u32; 2]>,
    values: Vec<u8>,
    valid: bool,
}

pub(super) struct Compiler {
    pub(super) circuit: Circuit,
    pairs: FxHashMap<[u32; 2], u32>,
}

impl Compiler {
    pub(super) fn new(leaves: usize) -> Self {
        // Literal zero is true and literal one is false.
        let mut values = vec![0; 2 * (1 + leaves)];
        values[0] = 1;
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
            let key = self.circuit.values.len() as u32;
            self.circuit.gates.push(pair);
            self.circuit.values.extend_from_slice(&[0, 1]);
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
        self.values.truncate(2 * (first + kept));
        map
    }

    pub(super) fn remap(map: &[u32], literal: u32) -> u32 {
        2 * map[literal as usize / 2] + (literal & 1)
    }

    pub(super) fn set(&mut self, index: usize, value: bool) -> bool {
        let i = 2 * (index + 1);
        let changed = self.values[i] != u8::from(value);
        self.values[i] = u8::from(value);
        self.values[i + 1] = u8::from(!value);
        changed
    }

    pub(super) fn evaluate(&mut self, changed: bool) {
        if self.valid && !changed { return; }
        let mut dest = 2 * (1 + self.leaves);
        for &[a, b] in &self.gates {
            let value = self.values[a as usize] & self.values[b as usize];
            self.values[dest] = value;
            self.values[dest + 1] = value ^ 1;
            dest += 2;
        }
        self.valid = true;
    }

    #[inline]
    pub(super) fn permits(&self, literal: u32) -> bool {
        self.values[literal as usize] != 0
    }

    pub(super) fn counts(&self) -> (usize, usize) { (self.leaves, self.gates.len()) }
}

#[cfg(test)]
mod tests {
    use super::Compiler;

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
            let mut changed = false;
            for (i, value) in bits.into_iter().enumerate() {
                changed |= circuit.set(i, value);
            }
            circuit.evaluate(changed);
            assert_eq!(circuit.permits(answer), bits[0] && (!bits[1] || bits[2]));
        }
    }
}
