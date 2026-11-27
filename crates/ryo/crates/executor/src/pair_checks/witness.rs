//! Demand only the neighbours of candidate nodes. Test results for the
//! entire conjunction are cached in the same slot as any other node mask.
use super::super::pair_summary::paths::Edge;
use super::*;

pub(crate) fn any(bits: &[u64], mut test: impl FnMut(u32) -> bool) -> bool {
    for (i, &v) in bits.iter().enumerate() {
        let mut word = v;
        while word != 0 {
            if test(i as u32 * 64 + word.trailing_zeros()) {
                return true;
            }
            word &= word - 1;
        }
    }
    false
}
pub(crate) fn contains(bits: &Nodes, id: u32) -> bool {
    bits.as_ref().is_none_or(|b| {
        b.get((id / 64) as usize)
            .is_some_and(|w| w & (1u64 << (id % 64)) != 0)
    })
}
impl File<'_> {
    /// The same lazy path test can answer either an existence probe or a
    /// request for all survivors. A shared conjunction materializes once.
    pub(crate) fn edge(&mut self, edge: Edge, output: Option<&mut Vec<u64>>) -> Option<bool> {
        let present = self.nodes(edge.candidates)?;
        if present.is_empty() {
            return Some(false);
        }
        let target = self.nodes(edge.targets);
        let unknown = target.is_none()
            && (edge.transitive || !matches!(self.p.domains[edge.targets], Domain::Any));
        let matches = |n: u32| {
            if unknown {
                return true;
            }
            if !edge.to_parent {
                return node_table::children_of(self.t, Position(n))
                    .iter()
                    .any(|p| contains(&target, p.0));
            }
            let mut id = n;
            loop {
                let parent = node_table::parent_of(self.t, Position(id)).0;
                if parent == id {
                    return false;
                }
                if contains(&target, parent) {
                    return true;
                }
                if !edge.transitive {
                    return false;
                }
                id = parent;
            }
        };
        match output {
            Some(out) => {
                each(&present, |n| {
                    if matches(n) {
                        insert(out, n)
                    }
                });
                Some(!out.is_empty())
            }
            None => Some(any(&present, matches)),
        }
    }
    pub(crate) fn path_exists(&mut self, x: Id, down: bool) -> bool {
        let Some(input) = self.nodes(x) else {
            return true;
        };
        any(&input, |i| {
            if down {
                !node_table::children_of(self.t, Position(i)).is_empty()
            } else {
                node_table::parent_of(self.t, Position(i)).0 != i
            }
        })
    }
}
