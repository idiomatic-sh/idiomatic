use super::*;

type Access = (usize, Option<usize>, Vec<usize>);
pub(super) struct Compiler<'a, 'p> {
    pub exec: &'a mut Exec<'p>,
    run: &'a mut Run,
    shared: FxHashSet<usize>,
    usable: Vec<bool>,
    leaves: FxHashMap<usize, Rc<Leaf>>,
    positives: FxHashMap<usize, Vec<usize>>,
    select: FxHashMap<usize, Rc<Select>>,
    pub cache: FxHashMap<Access, Option<Program>>,
}

// A range relation shared by distinct probes is an eligible shared input.
// Follow unary projections through different literal specializations to its
// union. Its own predecessor partitions are shared inputs as well.
fn shared_inputs(plan: &Plan) -> FxHashSet<usize> {
    fn compares<'a>(e: &'a J, out: &mut Vec<&'a J>) {
        match e[0].as_str() {
            Some("and" | "or") => {
                compares(&e[1], out);
                compares(&e[2], out);
            }
            Some("cmp") if e[2][0] == "col" && e[3][0] == "col" => out.push(e),
            _ => {}
        }
    }
    let mut users = FxHashMap::<usize, FxHashSet<usize>>::default();
    for n in &plan.nodes {
        if n.op != "select" || n.inputs.len() != 2 {
            continue;
        }
        let mut terms = Vec::new();
        compares(&n.j["cond"], &mut terms);
        for s in 0..2 {
            let alias = &n.j["src"][s][1];
            if terms.iter().any(|x| {
                x[1] == ">="
                    && &x[3][2] == alias
                    && x[2][2] != *alias
                    && terms
                        .iter()
                        .any(|y| y[1] == "<=" && y[2] == x[2] && &y[3][2] == alias && y[3] != x[3])
            }) {
                let source = n.inputs[s];
                let mut j = source;
                while plan.nodes[j].op == "select" && plan.nodes[j].inputs.len() == 1 {
                    j = plan.nodes[j].inputs[0]
                }
                if plan.nodes[j].op == "union" {
                    users.entry(j).or_default().insert(source);
                }
            }
        }
    }
    let mut cuts: FxHashSet<_> = users
        .into_iter()
        .filter(|(_, u)| u.len() > 1)
        .map(|(j, _)| j)
        .collect();
    let mut seen = FxHashSet::default();
    let mut q = cuts.iter().copied().collect::<Vec<_>>();
    while let Some(i) = q.pop() {
        if !seen.insert(i) {
            continue;
        }
        let n = &plan.nodes[i];
        if n.op == "as-of" {
            cuts.insert(n.j["args"]["right"]["rel"].as_u64().unwrap() as usize);
        }
        q.extend(&n.inputs);
    }
    cuts
}
fn index(sc: &Schema, value: &J) -> Option<usize> {
    pos(&sc.names, value.as_str()?).ok()
}
impl<'a, 'p> Compiler<'a, 'p> {
    pub fn new(exec: &'a mut Exec<'p>, run: &'a mut Run) -> Self {
        let shared = shared_inputs(exec.plan);
        let mut usable = Vec::new();
        for (i, n) in exec.plan.nodes.iter().enumerate() {
            let ok = shared.contains(&i)
                || (n.inputs.iter().all(|&j| usable[j])
                    && match n.op.as_str() {
                        "select" | "base" | "project" | "union" | "antijoin" | "axis"
                        | "extend" | "span-bounds" | "span-between" => true,
                        "as-of" => shared
                            .contains(&(n.j["args"]["right"]["rel"].as_u64().unwrap() as usize)),
                        _ => false,
                    });
            usable.push(ok);
        }
        Self {
            exec,
            run,
            shared,
            usable,
            leaves: Default::default(),
            positives: Default::default(),
            select: Default::default(),
            cache: Default::default(),
        }
    }
    fn sc(&mut self, i: usize) -> Option<Rc<Schema>> {
        self.exec.schema(self.run, i).ok()
    }
    fn leaf(&mut self, i: usize) -> Option<Rc<Leaf>> {
        if let Some(l) = self.leaves.get(&i) {
            return Some(l.clone());
        }
        let source = if self.shared.contains(&i) {
            Source::Shared
        } else if let Some(v) = self.exec.view(self.run, i).ok()? {
            Source::View(v)
        } else if self.exec.plan.nodes[i].op == "base" {
            Source::Relation(self.exec.plan.nodes[i].j["rel"].as_str()?.into())
        } else {
            return None;
        };
        let leaf = Rc::new(Leaf {
            id: i,
            source,
            schema: self.sc(i)?,
        });
        self.leaves.insert(i, leaf.clone());
        Some(leaf)
    }
    fn drivers(&mut self, i: usize) -> Vec<usize> {
        if let Some(d) = self.positives.get(&i) {
            return d.clone();
        }
        let n = &self.exec.plan.nodes[i];
        let mut v = if self.shared.contains(&i) {
            vec![]
        } else {
            match n.op.as_str() {
                "base" => {
                    if self.sc(i).is_some_and(|s| s.tys.contains(&Ty::Node)) {
                        vec![i]
                    } else {
                        vec![]
                    }
                }
                "select" => {
                    let mut v = self.drivers(n.inputs[0]);
                    if n.inputs.len() == 2 {
                        v.extend(self.drivers(n.inputs[1]));
                    } else if self.exec.plan.nodes[n.inputs[0]].op == "base"
                        && !v.is_empty()
                        && !n.j["cond"].is_null()
                        && self.leaf(i).is_some()
                    {
                        v.push(i)
                    }
                    v
                }
                "axis" => {
                    let mut v = self.drivers(n.j["args"]["from"]["rel"].as_u64().unwrap() as usize);
                    v.extend(self.drivers(n.j["args"]["domain"]["rel"].as_u64().unwrap() as usize));
                    v
                }
                "union" => {
                    let l = self.drivers(n.inputs[0]);
                    let r = self.drivers(n.inputs[1]);
                    l.into_iter().filter(|i| r.contains(i)).collect()
                }
                "as-of" | "antijoin" => {
                    self.drivers(n.j["args"]["left"]["rel"].as_u64().unwrap() as usize)
                }
                _ => self.drivers(n.j["args"]["from"]["rel"].as_u64().unwrap() as usize),
            }
        };
        v.sort_unstable();
        v.dedup();
        self.positives.insert(i, v.clone());
        v
    }
    pub fn rule(&mut self, emit: usize) -> Option<Rule> {
        let root = self.exec.plan.emits[emit].1;
        if !self.usable[root] {
            return None;
        }
        let mut choices = Vec::new();
        for d in self.drivers(root) {
            if let Some(path) = self.route(root, Some(d), vec![]) {
                choices.push(Choice {
                    leaf: self.leaf(d)?,
                    path,
                })
            }
        }
        choices.sort_by_key(|c| match &c.leaf.source {
            Source::View(v) => (
                v.filter.is_empty(),
                matches!(v.base, Native::Fact(_)),
                c.leaf.id,
            ),
            _ => (true, true, c.leaf.id),
        });
        (!choices.is_empty()).then(|| Rule {
            choices,
            schema: self.sc(root).unwrap(),
        })
    }
    fn edge(&mut self, i: usize, driver: Option<usize>, args: Args) -> Option<Edge> {
        Some(Edge {
            code: self.route(i, driver, args.iter().map(|x| x.0).collect())?,
            args: args.iter().map(|x| x.1).collect(),
        })
    }
    fn route(&mut self, i: usize, driver: Option<usize>, cols: Vec<usize>) -> Option<Program> {
        if driver.is_none() && cols.is_empty() {
            return None;
        }
        if let Some(d) = driver
            && !self.drivers(i).contains(&d)
        {
            return None;
        }
        let key = (i, driver, cols.clone());
        if let Some(r) = self.cache.get(&key) {
            return r.clone();
        }
        let r = self
            .instruction(i, driver, &cols)
            .map(|inst| Rc::new(Route { cols, inst }));
        self.cache.insert(key, r.clone());
        r
    }
    fn instruction(&mut self, i: usize, d: Option<usize>, cols: &[usize]) -> Option<Inst> {
        if let Some(l) = self.leaf(i) {
            // The access column depends on the compiled schema, never on a row.
            let probe = if d.is_some() {
                None
            } else {
                let (parameter, &column) = cols.iter().enumerate().min_by_key(|(_, c)| {
                    let ty = l.schema.tys[**c];
                    let native =
                        matches!(&l.source,Source::View(v) if File::probes(v.base,v.map[**c]));
                    (ty == Ty::File, !native, ty != Ty::Node, ty != Ty::Span)
                })?;
                Some(Probe { parameter, column })
            };
            return Some(Inst::Leaf(l, probe));
        }
        let n = &self.exec.plan.nodes[i];
        let a = &n.j["args"];
        let rel = |k: &str| a[k]["rel"].as_u64().map(|v| v as usize);
        let mut scs = Vec::new();
        for &j in &n.inputs {
            scs.push(self.sc(j)?)
        }
        let schema = self.sc(i)?;
        let outer = |sc: &Schema| -> Args {
            cols.iter()
                .enumerate()
                .filter_map(|(z, &c)| {
                    pos(&sc.names, &schema.names[c])
                        .ok()
                        .map(|k| (k, Arg::Outer(z)))
                })
                .collect()
        };
        Some(match n.op.as_str() {
            "select" => {
                let sp = self.exec.sel(self.run, i).ok()?;
                let sel = if let Some(s) = self.select.get(&i) {
                    s.clone()
                } else {
                    let s = Rc::new(Select::bind(&sp)?);
                    self.select.insert(i, s.clone());
                    s
                };
                let mut args: [BTreeMap<usize, Arg>; 2] = Default::default();
                for (idx, &c) in cols.iter().enumerate() {
                    if let E::Col(s, k, _) = sp.items[c].1 {
                        args[s].insert(k, Arg::Outer(idx));
                    }
                }
                let sources = &n.inputs;
                if sources.len() == 1 {
                    return Some(Inst::Filter(
                        self.edge(sources[0], d, args[0].clone().into_iter().collect())?,
                        sel,
                    ));
                }
                let mut eq = Vec::new();
                // The table planner elides equal-file predicates once it knows both
                // inputs are file-local. Keep those keys for navigation.
                let names = sources
                    .iter()
                    .enumerate()
                    .map(|(s, _)| (n.j["src"][s][1].as_str().map(str::to_owned), scs[s].clone()))
                    .collect::<Vec<_>>();
                let mut conditions = Vec::new();
                if !n.j["cond"].is_null() {
                    flatten(
                        self.exec.expr(self.run, &n.j["cond"], &names).ok()?,
                        &mut conditions,
                    );
                }
                for e in &conditions {
                    if let E::Cmp(Op::Eq, l, r) = e
                        && let (E::Col(s, x, t), E::Col(v, y, u)) = (&**l, &**r)
                        && s != v
                        && t == u
                    {
                        eq.push(if *s == 0 { [*x, *y] } else { [*y, *x] })
                    }
                }
                for _ in 0..eq.len() {
                    for &[l, r] in &eq {
                        if let Some(&v) = args[0].get(&l) {
                            args[1].entry(r).or_insert(v);
                        }
                        if let Some(&v) = args[1].get(&r) {
                            args[0].entry(l).or_insert(v);
                        }
                    }
                }
                let mut order = [0, 1];
                order.sort_by_key(|&s| {
                    let t = 1 - s;
                    let rank = |ty: &Ty| match ty {
                        Ty::Node => 0,
                        Ty::Span => 1,
                        Ty::Num | Ty::Sym => 2,
                        _ => 3,
                    };
                    let first = args[s]
                        .keys()
                        .map(|&k| rank(&scs[s].tys[k]))
                        .min()
                        .unwrap_or(4);
                    let second = eq
                        .iter()
                        .map(|v| rank(&scs[t].tys[v[t]]))
                        .min()
                        .unwrap_or(4);
                    (
                        if d.is_some() { 0 } else { first },
                        second,
                        self.leaves.contains_key(&sources[t]) == false,
                        s,
                    )
                });
                let mut result = None;
                for s in order {
                    let t = 1 - s;
                    let mut keys = args[t].clone();
                    for v in &eq {
                        keys.insert(v[t], Arg::First(v[s]));
                    }
                    if let Some(one) =
                        self.edge(sources[s], d, args[s].clone().into_iter().collect())
                        && let Some(two) = self.edge(sources[t], None, keys.into_iter().collect())
                    {
                        result = Some(Inst::Join(
                            one,
                            two,
                            s,
                            sel.clone(),
                            s == 0
                                && n.j.get("semi").is_some()
                                && sp.items.iter().all(|(_, e)| refs_of(e) & 2 == 0),
                        ));
                        break;
                    }
                }
                result?
            }
            "union" => Inst::Union(
                self.route(rel("left")?, d, cols.to_vec())?,
                self.route(rel("right")?, d, cols.to_vec())?,
            ),
            "antijoin" => {
                let l = rel("left")?;
                let r = rel("right")?;
                let cols_of = |sc: &Schema, x: &J| {
                    x.as_array()?
                        .iter()
                        .map(|n| index(sc, n))
                        .collect::<Option<Vec<_>>>()
                };
                let keys = cols_of(self.sc(l)?.as_ref(), &a["left-on"])?;
                let rk = cols_of(self.sc(r)?.as_ref(), &a["right-on"])?;
                Inst::Anti(
                    self.route(l, d, cols.to_vec())?,
                    self.route(r, None, rk)?,
                    keys,
                )
            }
            "axis" => {
                let from = rel("from")?;
                let dom = rel("domain")?;
                let relation = rel("relation")?;
                let start = self.sc(from)?;
                let domain_id = pos(&self.sc(dom)?.names, "id").ok()?;
                let id = pos(&start.names, "id").ok()?;
                let sr = self.sc(relation)?;
                let parent = pos(&sr.names, "parent").ok()?;
                let child = pos(&sr.names, "child").ok()?;
                let up = self.route(relation, None, vec![child])?;
                let down = self.route(relation, None, vec![parent])?;
                let dir = match a["direction"].as_str()? {
                    "ancestor" => Direction::Up,
                    "descendant" => Direction::Down,
                    "nearest-ancestor" => Direction::Nearest,
                    "parent" => Direction::Parent,
                    "child" => Direction::Child,
                    _ => return None,
                };
                let domain_check = self.route(dom, None, vec![domain_id])?;
                let (side, other, inverse) = if let Some(edge) = self.edge(from, d, outer(&start)) {
                    (edge, domain_check.clone(), false)
                } else {
                    let ak = cols
                        .iter()
                        .position(|&k| k == start.names.len())
                        .map(|z| vec![(domain_id, Arg::Outer(z))])
                        .unwrap_or_default();
                    (
                        self.edge(dom, d, ak)?,
                        self.route(from, None, vec![id])?,
                        true,
                    )
                };
                Inst::Axis(Axis {
                    side,
                    other,
                    id,
                    domain_id,
                    inverse,
                    dir,
                    up,
                    down,
                    parent,
                    child,
                    domain_check,
                })
            }
            "as-of" => {
                if a["bound"] != "at-or-before" {
                    return None;
                }
                let l = rel("left")?;
                let r = rel("right")?;
                let left = self.sc(l)?;
                let right = self.sc(r)?;
                let project: Vec<_> = schema
                    .names
                    .iter()
                    .map(|name| {
                        let pick = &a["output"][name];
                        let is_left = pick[0] == "left";
                        Some((
                            is_left,
                            index(if is_left { &left } else { &right }, &pick[1])?,
                        ))
                    })
                    .collect::<Option<_>>()?;
                let args = cols
                    .iter()
                    .enumerate()
                    .filter_map(|(z, &k)| project[k].0.then_some((project[k].1, Arg::Outer(z))))
                    .collect();
                let partition: Vec<(usize, usize)> = a["left-on"]
                    .as_array()?
                    .iter()
                    .zip(a["right-on"].as_array()?)
                    .map(|(x, y)| Some((index(&left, x)?, index(&right, y)?)))
                    .collect::<Option<_>>()?;
                Inst::Predecessor(Predecessor {
                    index: (
                        r,
                        index(&right, &a["right-order"])?,
                        partition.iter().map(|&(_, c)| c).collect(),
                    ),
                    input: self.edge(l, d, args)?,
                    left_order: index(&left, &a["left-order"])?,
                    partition,
                    project,
                })
            }
            _ => {
                let j = rel("from")?;
                let sc = self.sc(j)?;
                let t = match n.op.as_str() {
                    "project" => Transform::Columns(
                        schema
                            .names
                            .iter()
                            .map(|n| pos(&sc.names, n).ok())
                            .collect::<Option<_>>()?,
                    ),
                    "extend" => Transform::Append(
                        schema.names[sc.names.len()..]
                            .iter()
                            .map(|k| {
                                let v = &a["values"][k];
                                if let Some(q) = v.as_i64() {
                                    Some(q as u64)
                                } else {
                                    Some(match v.as_str()? {
                                        "true" => 1,
                                        "false" => 0,
                                        t => self.run.sym(t.as_bytes()).into(),
                                    })
                                }
                            })
                            .collect::<Option<_>>()?,
                    ),
                    "span-bounds" => Transform::Bounds(index(&sc, &a["span"])?),
                    "span-between" => {
                        Transform::Span(index(&sc, &a["start"])?, index(&sc, &a["end"])?)
                    }
                    _ => return None,
                };
                Inst::Transform(self.edge(j, d, outer(&sc))?, t)
            }
        })
    }
}
