//! A closed equality can get its node keys from a file-local posting. For an
//! axis, every returned "related" node belongs to the axis's declared domain.
use super::super::{E, Exec, Op, Run, Ty};
use std::collections::{BTreeMap, BTreeSet};

pub(in super::super::super) fn domains(
    exec: &mut Exec<'_>,
    run: &mut Run,
    local: &[bool],
    inputs: &BTreeSet<usize>,
) -> BTreeMap<usize, (usize, usize)> {
    let mut found = BTreeMap::new();
    for &source in inputs {
        if exec.plan.emits.iter().any(|e| e.1 == source) {
            continue;
        }
        let uses = exec.consumers[source].clone();
        let mut binding = None;
        let bounded = uses.into_iter().filter(|&u| !local[u]).all(|u| {
            if exec.plan.nodes[u].op != "select" {
                return false;
            }
            let Ok(sel) = exec.sel(run, u) else {
                return false;
            };
            if sel.srcs.len() != 2
                || sel.srcs[0] == sel.srcs[1]
                || !sel
                    .conj
                    .iter()
                    .chain(sel.items.iter().map(|(_, e)| e))
                    .all(super::certificate::total)
            {
                return false;
            }
            let side = usize::from(sel.srcs[1] == source);
            let other = sel.srcs[1 - side];
            let key = sel.conj.iter().find_map(|term| {
                let E::Cmp(Op::Eq, a, b) = term else {
                    return None;
                };
                let (E::Col(x, i, Ty::Node), E::Col(y, j, Ty::Node)) = (&**a, &**b) else {
                    return None;
                };
                if x == y {
                    return None;
                }
                let (col, driver) = if *x == side { (*i, *j) } else { (*j, *i) };
                let schema = exec.schema(run, other).ok()?;
                let domain = if local[other] && schema.names[driver] == "id" {
                    other
                } else if exec.plan.nodes[other].op == "axis"
                    && driver + 1 == schema.names.len()
                    && schema.names[driver] == "related"
                {
                    exec.arg_rel(other, "domain").ok()?
                } else {
                    return None;
                };
                if !local[domain] {
                    return None;
                }
                Some((col, domain))
            });
            key.is_some_and(|k| *binding.get_or_insert(k) == k)
        });
        if bounded && let Some(key) = binding {
            found.insert(source, key);
        }
    }
    found
}
