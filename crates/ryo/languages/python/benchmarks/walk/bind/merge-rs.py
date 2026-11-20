#!/usr/bin/env python3
"""Resolves the 14 conflict hunks of nav.rs and the one of rules.rs of the merge claude/sw5-cut into claude/sw6-bind.
Each hunk: 'ours' (asof), 'theirs' (cut), 'both' (ours then theirs), or a literal text."""
import re, sys
def split(path):
    out, cur, state = [], None, 0
    for line in open(path):
        if line.startswith('<<<<<<< '):
            cur = {'ours': [], 'theirs': []}; state = 1
        elif line.startswith('=======') and state == 1:
            state = 2
        elif line.startswith('>>>>>>> ') and state == 2:
            out.append(cur); cur = None; state = 0
        elif state == 1: cur['ours'].append(line)
        elif state == 2: cur['theirs'].append(line)
        else: out.append(line)
    return out
def join(parts, res):
    k = 0; text = []
    for p in parts:
        if isinstance(p, str): text.append(p); continue
        r = res[k]; k += 1
        o, t = ''.join(p['ours']), ''.join(p['theirs'])
        text.append(r(o, t) if callable(r) else {'ours': o, 'theirs': t, 'both': o + t}[r] if r in ('ours', 'theirs', 'both') else r)
    assert k == len(res), (k, len(res))
    return ''.join(text)

walk = sys.argv[1]
nav = split(walk + '/nav.rs')
def h0(o, t):
    # the instruction enum: asof's table use on the keyed calls plus cut's args; cut's probes; asof's keyed iteration and tables
    o = o.replace("UnionKeyed { outs: Box<[R]>, subs: Box<[Prog]>, keys: Box<[R]>, table: Option<TableUse> },",
                  "/// cut: `args`, when given, are written into the `keys` registers by the instruction itself (the call passes its\n    /// arguments in one step instead of one `let` each); asof: `table`, how the call reads its definition's table\n    UnionKeyed { outs: Box<[R]>, subs: Box<[Prog]>, keys: Box<[R]>, args: Box<[Opd]>, table: Option<TableUse> },")
    o = o.replace("SubKeyed { negate: bool, subs: Box<[Prog]>, keys: Box<[R]>, table: Option<TableUse> },",
                  "SubKeyed { negate: bool, subs: Box<[Prog]>, keys: Box<[R]>, args: Box<[Opd]>, table: Option<TableUse> },")
    probes = t[t.index("    /// cut: a call of a definition entered"):t.index("}\n")]
    i = o.index("}\n")
    o = o[:i] + probes + o[i:]
    tail = t[t.index("/// cut: a definition's uncorrelated rows indexed"):]
    return o + "}\n\n" + tail
def keyed_loader(o, t):
    return t.rstrip('\n').rsplit('\n', 0)[0] + '\n'
res_nav = [
    h0,
    # neg-keyed loader: args at 3, table at 4
    "                args: a.get(3).and_then(Value::as_array).map(|v| v.iter().map(|x| self.opd(x)).collect::<Vec<_>>()).unwrap_or_default().into(),\n                table: self.table_use(a.get(4)),\n",
    # or-keyed loader: args at 4, table at 5; then cut's probes
    lambda o, t: t.replace("unwrap_or_default().into(),\n            },", "unwrap_or_default().into(),\n                table: self.table_use(a.get(5)),\n            },", 1),
    # counters: cut's profile, plus asof's hit and table counters
    lambda o, t: t + "    /// asof: keyed memo hits; definition tables built and abandoned (budget)\n    pub keyed_hits: u64,\n    pub table_builds: u64,\n    pub table_fails: u64,\n",
    'both',  # Facts: kidx and midx
    'both',  # Exec fields
    'both',  # ensure: kidx and midx
    # made table: asof's abort, then cut's timer
    'both',
    # fact_index: not loaded -> empty, then timer
    'both',
    # IterFactKeyed (asof) and IterFactMulti (cut): two arms
    None, None,
    # UnionKeyed / SubKeyed arms
    """                Ins::UnionKeyed { outs, subs, keys, args, table } => {
                    for (k, a) in keys.iter().zip(args.iter()) {
                        self.regs[*k as usize] = self.opd(a);
                    }
                    return self.union_keyed(p, pc, outs, subs, keys, table.as_ref());
                }
                Ins::SubKeyed { negate, subs, keys, args, table } => {
                    for (k, a) in keys.iter().zip(args.iter()) {
                        self.regs[*k as usize] = self.opd(a);
                    }
                    let any = self.sub_keyed(subs, keys, table.as_ref());
                    if self.aborted {
                        return true;
                    }
                    if any == *negate {
""",
    'theirs',  # Shared: cut's profile
    # union_keyed miss: asof's abort, cut's one-row shortcut, asof's vals
    """                if self.aborted {
                    return true;
                }
                let mut distinct: Vec<u64> = vals.clone();
                if rows.len() <= w {
                    // cut: none or one row is distinct as it stands (no set)
                    distinct.extend_from_slice(&rows);
                } else {
                    let mut seen: std::collections::HashSet<&[u64], std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
                    for row in rows.chunks(w) {
                        if seen.insert(row) {
                            distinct.extend_from_slice(row);
                        }
""",
]
# hunks 9 and 10 are one interleaved pair (two different arms git aligned): write both arms whole
h9 = nav_h = [p for p in nav if not isinstance(p, str)]
ours9, theirs9, ours10, theirs10 = ''.join(h9[9]['ours']), ''.join(h9[9]['theirs']), ''.join(h9[10]['ours']), ''.join(h9[10]['theirs'])
# the common text between hunk 9 and hunk 10
idx = [i for i, p in enumerate(nav) if not isinstance(p, str)]
between = ''.join(nav[idx[9] + 1:idx[10]])
arm_asof = ours9 + between + ours10
arm_cut = theirs9 + between + theirs10
res_nav[9] = arm_asof + "                }\n" + arm_cut
res_nav[10] = ""
# drop the common text between the two hunks (it is inside both arms now)
for j in range(idx[9] + 1, idx[10]):
    nav[j] = ""
text = join(nav, res_nav)
open(walk + '/nav.rs', 'w').write(text)
rules = split(walk + '/rules.rs')
open(walk + '/rules.rs', 'w').write(join(rules, ['both']))
