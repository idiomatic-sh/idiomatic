//! Demand only the neighbours of candidate nodes. Test results for the
//! entire conjunction are cached in the same slot as any other node mask.
use super::super::pair_summary::paths::Edge;
use super::*;

fn any(bits: &[u64], mut test: impl FnMut(u32) -> bool) -> bool {
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
fn contains(arena: &Arena, bits: Nodes, id: u32) -> bool {
    bits.is_none_or(|b| arena.word(b, (id / 64) as usize) & (1u64 << (id % 64)) != 0)
}
impl<const PROFILE: bool> File<'_, PROFILE> {
    /// The same lazy path test can answer either an existence probe or a
    /// request for all survivors. A shared conjunction materializes once.
    pub(super) fn edge(&mut self, edge: Edge, output: Option<&mut Vec<u64>>) -> Option<bool> {
        let present = self.nodes(edge.candidates)?;
        if present.is_empty() {
            return Some(false);
        }
        let target = self.nodes(edge.targets);
        let unknown = target.is_none()
            && (edge.transitive || !matches!(self.p.domains[edge.targets], Domain::Any));
        let offset = present.first() as u32 * 64;
        let present = &self.arena.words[present.range()];
        let matches = |n: u32| {
            if unknown {
                return true;
            }
            if !edge.to_parent {
                return node_table::children_of(self.t, Position(n))
                    .iter()
                    .any(|p| contains(self.arena, target, p.0));
            }
            let mut id = n;
            loop {
                let parent = node_table::parent_of(self.t, Position(id)).0;
                if parent == id {
                    return false;
                }
                if contains(self.arena, target, parent) {
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
                each(present, |local| {
                    let n = offset + local;
                    if matches(n) {
                        insert(out, n)
                    }
                });
                Some(!out.is_empty())
            }
            None => Some(any(present, |local| matches(offset + local))),
        }
    }
    pub(super) fn path_exists(&mut self, x: Id, down: bool) -> bool {
        let Some(input) = self.nodes(x) else {
            return true;
        };
        any(&self.arena.words[input.range()], |local| {
            let i = input.first() as u32 * 64 + local;
            if down {
                !node_table::children_of(self.t, Position(i)).is_empty()
            } else {
                node_table::parent_of(self.t, Position(i)).0 != i
            }
        })
    }
}
