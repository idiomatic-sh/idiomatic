//! THROWAWAY (ryo-fmx3.1, step 6e item 2): the prefix hands over every variable it bound, not only its root. The prefix
//! is an existential filter: a register bound by a move down (a child) holds the first witness only, so the registers
//! themselves would lose rows. What is handed over instead is exact and complete: the roots that survived, and per seed
//! set the move paths (parent, child, ancestor) from a plan's root to each of its other variables; the set is the roots
//! and every node those paths reach from them.

use super::File;
use ryo::languages::{Position, node_table};

/// Every node the moves reach from `from` (all of them: the prefix's conditions after a move are not re-applied, so this is
/// a superset of the variable's values in any finding).
fn step(f: &File<'_>, from: &[u32], mv: &str, out: &mut Vec<u32>) {
    for &v in from {
        match mv {
            "up" => {
                let p = f.parent[v as usize];
                if p != v {
                    out.push(p);
                }
            }
            "down" => {
                for c in node_table::children_of(f.table, Position(v)) {
                    if c.0 != v {
                        out.push(c.0);
                    }
                }
            }
            _ => {
                let mut n = v;
                loop {
                    let p = f.parent[n as usize];
                    if p == n {
                        break;
                    }
                    out.push(p);
                    n = p;
                }
            }
        }
    }
}

/// Widens each seed set of the file by its move paths (sets with no path keep the roots).
pub(crate) fn expand(f: &mut File<'_>, paths: &[Vec<Vec<String>>]) {
    for (k, ps) in paths.iter().enumerate() {
        if ps.is_empty() || k >= f.seeds.len() || f.seeds[k].is_empty() {
            continue;
        }
        let roots = f.seeds[k].clone();
        let mut all = roots.clone();
        for p in ps {
            let mut cur = roots.clone();
            for mv in p {
                let mut next = Vec::new();
                step(f, &cur, mv, &mut next);
                next.sort_unstable();
                next.dedup();
                cur = next;
            }
            all.extend_from_slice(&cur);
        }
        all.sort_unstable();
        all.dedup();
        f.seeds[k] = all;
    }
}
