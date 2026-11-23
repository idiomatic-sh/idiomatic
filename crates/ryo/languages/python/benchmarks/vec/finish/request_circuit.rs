//! File predicates have fixed dependencies. Compile their conjunctions and
//! disjunctions once, sharing equal pairs. Literals are adjacent truth and
//! complement bytes, so each gate needs two indexed reads and one AND.
use rustc_hash::FxHashMap;

#[derive(Default)]
pub(super) struct Circuit {
    pub(super) leaves: Vec<u32>,
    gates: Vec<[u32; 2]>,
    values: Vec<u8>,
    valid: bool,
}

pub(super) struct Compiler {
    pub(super) circuit: Circuit,
    pairs: FxHashMap<[u32; 2], u32>,
}

impl Compiler {
    pub(super) fn new(leaves: Vec<u32>) -> Self {
        // Literal zero is true and literal one is false.
        let mut values = vec![0; 2 * (1 + leaves.len())];
        values[0] = 1;
        Self { circuit: Circuit { leaves, values, ..Default::default() }, pairs: FxHashMap::default() }
    }

    pub(super) fn and(&mut self, a: u32, b: u32) -> u32 {
        if a == 0 || a == b { return b; }
        if b == 0 { return a; }
        if a == 1 || b == 1 || a == (b ^ 1) { return 1; }
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
    pub(super) fn set(&mut self, index: usize, value: bool) -> bool {
        let i = 2 * (index + 1);
        let changed = self.values[i] != u8::from(value);
        self.values[i] = u8::from(value);
        self.values[i + 1] = u8::from(!value);
        changed
    }

    pub(super) fn evaluate(&mut self, changed: bool) {
        if self.valid && !changed { return; }
        let mut dest = 2 * (1 + self.leaves.len());
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

    pub(super) fn counts(&self) -> (usize, usize) { (self.leaves.len(), self.gates.len()) }
}
