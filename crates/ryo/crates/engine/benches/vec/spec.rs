//! Typed prefix plans and per-file word requirements.

/// The bucket a plan is entered at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bucket {
    Kinds,
    Fields,
    Words,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SpecCheck {
    pub(super) v: usize,
    pub(super) kinds: Option<Vec<String>>,
    pub(super) notkinds: Option<Vec<String>>,
    pub(super) texts: Option<Vec<Vec<u8>>>,
    pub(super) fields: Option<Vec<String>>,
    pub(super) re: Option<String>,
}

/// A fact test: the relation, its (key column, register) pairs and its (column, op, values) conditions.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SpecFact {
    pub(super) rel: String,
    pub(super) keys: Vec<(String, usize)>,
    pub(super) conds: Vec<(String, String, Vec<String>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum SpecOp {
    Check(SpecCheck),
    Up {
        from: usize,
        to: usize,
        leaf: bool,
    },
    Down {
        from: usize,
        to: usize,
        leaf: bool,
    },
    Not(Vec<SpecOp>),
    Exists {
        from: usize,
        to: usize,
        plan: Vec<SpecOp>,
    },
    IsParent {
        child: usize,
        parent: usize,
    },
    Fact(SpecFact),
    Enc {
        from: usize,
        to: usize,
        kinds: Vec<String>,
    },
    AncX {
        from: usize,
        to: usize,
        plan: Vec<SpecOp>,
    },
}

/// One plan: the disjunct of its rule it enters (plans of one disjunct are alternatives: each file enters at one of them),
/// its bucket and keys, its ops and registers, and its required-word CNF.
#[derive(Clone, Debug)]
pub(super) struct SpecPlan {
    pub(super) disjunct: usize,
    pub(super) bucket: Bucket,
    pub(super) keys: Vec<String>,
    pub(super) ops: Vec<SpecOp>,
    pub(super) regs: usize,
    pub(super) words: Vec<Vec<String>>,
}

impl SpecPlan {
    pub(super) fn same(&self, o: &SpecPlan) -> bool {
        self.bucket == o.bucket && self.keys == o.keys && self.ops == o.ops && self.words == o.words
    }
}

#[derive(Clone, Debug)]
pub(super) struct SpecRule {
    pub(super) name: String,
    /// Of the node class: its rows are found from tree survivors.
    pub(super) node: bool,
    pub(super) plans: Vec<SpecPlan>,
    pub(super) word_cnfs: Vec<Vec<Vec<String>>>,
}
