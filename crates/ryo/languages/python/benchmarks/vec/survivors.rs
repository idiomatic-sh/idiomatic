//! THROWAWAY (ryo-fmx3.1, redo-prefix change 4): the first half's result handed to the second half in memory, as typed
//! columns. Nothing is formatted while the files run; the text form is only a dump for comparing, written after.

/// The first half's result for a run: per (unit, file) with survivors, the unit's rule, the file and its survivors (in the
/// order the plans found them); the (rule, file) pairs with survivors; and the (rule, file) pairs the skip kept.
#[derive(Default)]
pub(super) struct Survivors {
    pub(super) rule: Vec<u32>,
    pub(super) file: Vec<u32>,
    /// Where each entry's survivors end in `nodes`; they start where the entry before ends.
    pub(super) end: Vec<u32>,
    pub(super) nodes: Vec<u32>,
    pub(super) hits: Vec<(u32, u32)>,
    pub(super) alive: Vec<(u32, u32)>,
}

impl Survivors {
    /// Ends one (unit, file) entry with the survivors pushed to `nodes` since the last entry; none, no entry.
    pub(super) fn close(&mut self, rule: usize, file: usize) {
        let start = self.end.last().copied().unwrap_or(0) as usize;
        if self.nodes.len() > start {
            self.rule.push(rule as u32);
            self.file.push(file as u32);
            self.end.push(u32::try_from(self.nodes.len()).expect("fewer than 2^32 survivors"));
        }
    }

    /// The second half's seed: every (rule, file) with survivors, and every pair the skip kept of a rule with no plan of
    /// the node class (its rows are not found from tree survivors); sorted, once each (as `scripts6g/fileseed.py`).
    pub(super) fn seed(&self, node_class: &[bool]) -> Vec<(u32, u32)> {
        let mut s: Vec<(u32, u32)> = self.hits.iter().copied().chain(self.alive.iter().copied().filter(|(r, _)| !node_class[*r as usize])).collect();
        s.sort_unstable();
        s.dedup();
        s
    }

    /// The survivors as text: a line an entry, `rule \t path \t node node ...`.
    pub(super) fn surv_text(&self, names: &[&str], paths: &[String]) -> String {
        use std::fmt::Write as _;
        let mut w = String::new();
        let mut start = 0;
        for (i, end) in self.end.iter().enumerate() {
            let _ = write!(w, "{}\t{}\t", names[self.rule[i] as usize], paths[self.file[i] as usize]);
            for (j, x) in self.nodes[start..*end as usize].iter().enumerate() {
                let _ = write!(w, "{}{x}", if j == 0 { "" } else { " " });
            }
            w.push('\n');
            start = *end as usize;
        }
        w
    }

    /// (rule, file) pairs as text: a line a pair, `rule \t path`.
    pub(super) fn pairs_text(pairs: &[(u32, u32)], names: &[&str], paths: &[String]) -> String {
        let mut w = String::new();
        for (r, f) in pairs {
            w.push_str(names[*r as usize]);
            w.push('\t');
            w.push_str(&paths[*f as usize]);
            w.push('\n');
        }
        w
    }
}
