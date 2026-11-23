//! Compile same-operation register trees to one short-circuit mask group.
//! Keep multiply read subtrees in their original shared register.
use super::pair_summary::{Domain, Id, Need, Program};
#[derive(Default)]
pub(super) struct Groups {
    pub(super) nodes: Vec<Vec<Id>>,
}
impl Groups {
    pub(super) fn new(p: &Program, cost: &[u8]) -> Self {
        let mut refs = vec![0usize; p.domains.len()];
        for d in &p.domains {
            match *d {
                Domain::Both(a, b)
                | Domain::Either(a, b)
                | Domain::Except(a, b)
                | Domain::ProvenExcept(a, b, ..) => {
                    refs[a] += 1;
                    refs[b] += 1;
                }
                Domain::When(_, a)
                | Domain::CountUnder(a, ..)
                | Domain::Children(a)
                | Domain::Parents(a)
                | Domain::Descendants(a)
                | Domain::Ancestors(a) => refs[a] += 1,
                _ => {}
            }
        }
        for n in &p.needs {
            if let Need::Some(x) = n {
                refs[*x] += 1;
            }
        }
        for (_, c) in &p.exact {
            refs[c.nodes] += 1;
        }
        let mut nodes = vec![Vec::new(); p.domains.len()];
        let mut visited = vec![usize::MAX; p.domains.len()];
        let mut todo = Vec::new();
        for (id, d) in p.domains.iter().enumerate() {
            let (and, a, b) = match *d {
                Domain::Both(a, b) => (true, a, b),
                Domain::Either(a, b) => (false, a, b),
                _ => continue,
            };
            let terms = &mut nodes[id];
            todo.extend([a, b]);
            while let Some(child) = todo.pop() {
                if visited[child] == id {
                    continue;
                }
                visited[child] = id;
                match p.domains[child] {
                    Domain::Both(a, b) if and && refs[child] == 1 => todo.extend([a, b]),
                    Domain::Either(a, b) if !and && refs[child] == 1 => todo.extend([a, b]),
                    _ => terms.push(child),
                }
            }
            if terms.len() < 3 {
                terms.clear();
                continue;
            }
            terms.sort_unstable_by_key(|&x| (cost[x], x));
        }
        Self { nodes }
    }
}
