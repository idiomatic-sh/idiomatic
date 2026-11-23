//! Per-rule word prerequisites supplied by the typed pair summary.
#[derive(Clone, Debug)]
pub(super) struct SpecRule {
    pub(super) name: String,
    pub(super) word_cnfs: Vec<Vec<Vec<String>>>,
}
