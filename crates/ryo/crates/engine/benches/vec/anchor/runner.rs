use super::super::super::thread_cpu as clock;
use super::*;

type OrderedPostings = FxHashMap<Row, Vec<(i64, usize)>>;
#[derive(Default)]
pub(crate) struct FileIndexes {
    ordered: FxHashMap<OrderedKey, Rc<OrderedPostings>>,
    tables: FxHashMap<usize, Rc<Table>>,
    // Extra postings for columns with no native access path.
    native_indexes: FxHashMap<(usize, usize), (Vec<u32>, Index)>,
    indexes: FxHashMap<(usize, usize), Rc<Index>>,
}
impl Programs {
    pub(crate) fn execute(
        &mut self,
        emit: usize,
        exec: &mut Exec<'_>,
        f: &mut File<'_>,
        run: &mut Run,
        pv: &mut dyn Provider,
        file: &mut FileIndexes,
    ) -> R<Rc<Table>> {
        let rule = self.rules[emit].as_ref().expect("compiled emit");
        let start = clock();
        self.stats.pairs += 1;
        let before = (self.stats.anchors, self.stats.probes, self.stats.visits);
        let mut ctx = Session {
            exec,
            f,
            run,
            pv,
            file,
            stat: &mut self.stats,
            anchor: 0,
        };
        let mut out = Table::with(rule.schema.names.clone(), rule.schema.tys.clone());
        let mut best = None;
        let mut len = usize::MAX;
        for choice in &rule.choices {
            let n = ctx.size(&choice.leaf)?;
            if n < len {
                best = Some(choice);
                len = n;
            }
            if n == 0 {
                break;
            }
        }
        ctx.stat.choice += clock() - start;
        if len > 0
            && let Some(choice) = best
        {
            ctx.stat.anchors += len as u64;
            let mut visit = |ctx: &mut Session<'_, '_, '_>, row| {
                ctx.anchor = row;
                ctx.visit(&choice.path, &[], &mut |_, r| {
                    out.push_row(r);
                    Ok(false)
                })?;
                Ok::<_, String>(())
            };
            if let Source::View(v) = &choice.leaf.source {
                if v.filter.is_empty()
                    && matches!(v.base, Native::Tree | Native::Location | Native::Fact(_))
                {
                    for row in 0..len as u32 {
                        visit(&mut ctx, row)?;
                    }
                } else {
                    let rows = ctx.population(&choice.leaf, v);
                    for &row in rows.iter() {
                        visit(&mut ctx, row)?;
                    }
                }
            } else {
                for row in 0..len as u32 {
                    visit(&mut ctx, row)?;
                }
            }
        }
        let elapsed = clock() - start;
        ctx.stat.total += elapsed;
        if let Some((_, anchors, probes, visits, secs)) = self.detail.get_mut(emit) {
            *anchors += ctx.stat.anchors - before.0;
            *probes += ctx.stat.probes - before.1;
            *visits += ctx.stat.visits - before.2;
            *secs += elapsed;
        }
        Ok(Rc::new(out))
    }
}

struct Session<'s, 'p, 'f> {
    exec: &'s mut Exec<'p>,
    f: &'s mut File<'f>,
    run: &'s mut Run,
    pv: &'s mut dyn Provider,
    file: &'s mut FileIndexes,
    stat: &'s mut Stats,
    anchor: u32,
}
fn args(e: &Edge, keys: &[u64], first: &[u64]) -> Row {
    e.args
        .iter()
        .map(|v| match *v {
            Arg::Outer(c) => keys[c],
            Arg::First(c) => first[c],
        })
        .collect()
}
impl<'f> Session<'_, '_, 'f> {
    fn table(&mut self, id: usize, relation: Option<&str>) -> R<Rc<Table>> {
        if let Some(t) = self.file.tables.get(&id) {
            return Ok(t.clone());
        }
        let start = clock();
        let t = if let Some(rel) = relation {
            Rc::new(self.exec.base(self.f, self.run, self.pv, rel)?)
        } else {
            self.exec.eval(self.f, self.run, self.pv, id)?
        };
        if relation.is_none() {
            self.stat.shared += clock() - start;
            self.stat.shared_rows += t.n as u64;
            self.stat.shared_builds += 1;
        } else {
            self.stat.fills += clock() - start;
        }
        self.file.tables.insert(id, t.clone());
        Ok(t)
    }
    fn ready(&mut self, n: Native) -> R<()> {
        if let Native::Fact(slot) = n
            && self.f.facts.get(slot as usize).is_none_or(Option::is_none)
        {
            let t = clock();
            self.exec.load_fact(self.f, self.run, self.pv, n)?;
            self.stat.fills += clock() - t;
        }
        Ok(())
    }
    fn size(&mut self, l: &Leaf) -> R<usize> {
        match &l.source {
            Source::View(v) => {
                self.ready(v.base)?;
                if v.filter.is_empty() {
                    match v.base {
                        Native::Tree | Native::Location => return Ok(self.f.n),
                        Native::Fact(i) => return Ok(self.f.facts[i as usize].as_ref().unwrap().n),
                        _ => {}
                    }
                }
                Ok(self.population(l, v).len())
            }
            Source::Relation(name) => Ok(self.table(l.id, Some(name))?.n),
            Source::Shared => Ok(self.table(l.id, None)?.n),
        }
    }
    fn population(&mut self, l: &Leaf, v: &ViewSpec) -> cold_bucket::Rows<'f> {
        if let Some(slot) = self.f.vrows.slot(&l.id) {
            return slot.resolve(self.f.table);
        }
        let start = clock();
        let fact_builds = self.f.fact_ix.borrow().len();
        let fact_rows = FACT_IX_ROWS.load(std::sync::atomic::Ordering::Relaxed);
        let rows = view_rows(self.f, self.run, v, l.id);
        let owned = matches!(
            self.f.vrows.slot(&l.id),
            Some(cold_bucket::Slot::Owned(_) | cold_bucket::Slot::Merge(_))
        );
        let builds = (self.f.fact_ix.borrow().len() - fact_builds) as u64 + u64::from(owned);
        if builds != 0 {
            self.stat.builds += builds;
            self.stat.rows += FACT_IX_ROWS.load(std::sync::atomic::Ordering::Relaxed) - fact_rows
                + if owned { rows.len() as u64 } else { 0 };
            if matches!(rows, cold_bucket::Rows::Merge(..)) {
                let _ = rows.iter();
            }
            self.stat.index += clock() - start;
        }
        rows
    }
    fn native_ids(&mut self, l: &Leaf, v: &ViewSpec, k: usize, value: u64) -> R<Vec<u32>> {
        self.ready(v.base)?;
        if File::probes(v.base, k) {
            let build = match v.base {
                Native::Fact(i) => !self.f.fact_ix.borrow().contains_key(&(i, k)),
                Native::Location => match k {
                    3 | 4 => self.f.by_line[k - 3].borrow().is_none(),
                    5 => self.f.by_start.borrow().is_none(),
                    6 => self.f.by_end.borrow().is_none(),
                    _ => false,
                },
                _ => false,
            };
            let t = if build { clock() } else { 0.0 };
            let mut rows = Vec::new();
            self.f.native_probe(self.run, v.base, k, value, &mut rows);
            if build {
                self.stat.index += clock() - t;
                self.stat.builds += 1;
                self.stat.rows += match v.base {
                    Native::Fact(i) => self.f.facts[i as usize].as_ref().unwrap().n,
                    _ => self.f.n,
                } as u64
            }
            Ok(rows)
        } else {
            // Build on the filtered physical row IDs; index values map back to that population.
            let key = (l.id, k);
            if !self.file.native_indexes.contains_key(&key) {
                let population = self.population(l, v);
                let t = clock();
                let ids = population.to_vec();
                let vals: Vec<_> = ids
                    .iter()
                    .map(|&r| self.f.native_get(self.run, v.base, r, k))
                    .collect();
                let index = Index::build(vals.len(), |j| vals[j]);
                self.stat.rows += ids.len() as u64;
                self.stat.builds += 1;
                self.stat.index += clock() - t;
                self.file.native_indexes.insert(key, (ids, index));
            }
            let (ids, index) = &self.file.native_indexes[&key];
            Ok(index
                .get(&value)
                .unwrap_or_default()
                .iter()
                .map(|&j| ids[j as usize])
                .collect())
        }
    }
    fn leaf(
        &mut self,
        l: &Leaf,
        probe: Option<Probe>,
        keys: &[u64],
        emit: &mut dyn FnMut(&mut Self, &[u64]) -> R<bool>,
    ) -> R<bool> {
        let drive = probe.is_none();
        if !drive {
            self.stat.probes += 1
        }
        let sample = !drive
            && self.stat.diagnostic
            && self.stat.probes.wrapping_mul(0x9e3779b97f4a7c15) >> 54 == 0;
        let start = if sample { clock() } else { 0.0 };
        match &l.source {
            Source::View(v) => {
                let rows = if let Some(p) = probe {
                    Some(self.native_ids(l, v, v.map[p.column], keys[p.parameter])?)
                } else {
                    None
                };
                if sample {
                    self.stat.probe_seconds += clock() - start;
                    self.stat.probe_samples += 1;
                }
                let one = [self.anchor];
                for &r in rows.as_deref().unwrap_or(&one) {
                    if !view_holds(self.f, self.run, v, r) {
                        continue;
                    }
                    let cells: Row = v
                        .map
                        .iter()
                        .map(|&c| self.f.native_get(self.run, v.base, r, c))
                        .collect();
                    if emit(self, &cells)? {
                        return Ok(true);
                    }
                }
            }
            src => {
                let t = self.table(
                    l.id,
                    match src {
                        Source::Relation(rel) => Some(rel),
                        _ => None,
                    },
                )?;
                let one = [self.anchor];
                let ix;
                let ids = if drive {
                    &one[..]
                } else {
                    let Probe {
                        parameter: z,
                        column: c,
                    } = probe.expect("keyed leaf");
                    ix = if let Some(ix) = self.file.indexes.get(&(l.id, c)) {
                        ix.clone()
                    } else {
                        let now = clock();
                        let ix = Rc::new(Index::build(t.n, |r| t.cols[c][r]));
                        self.stat.rows += t.n as u64;
                        self.stat.builds += 1;
                        self.stat.index += clock() - now;
                        self.file.indexes.insert((l.id, c), ix.clone());
                        ix
                    };
                    ix.get(&keys[z]).unwrap_or_default()
                };
                if sample {
                    self.stat.probe_seconds += clock() - start;
                    self.stat.probe_samples += 1;
                }
                for &r in ids {
                    let cells: Row = t.cols.iter().map(|c| c[r as usize]).collect();
                    if emit(self, &cells)? {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
    fn visit(
        &mut self,
        code: &Route,
        keys: &[u64],
        sink: &mut dyn FnMut(&mut Self, &[u64]) -> R<bool>,
    ) -> R<bool> {
        self.stat.visits += 1;
        let mut emit = |ctx: &mut Self, row: &[u64]| {
            if code.cols.iter().zip(keys).all(|(&c, k)| row[c] == *k) {
                sink(ctx, row)
            } else {
                Ok(false)
            }
        };
        match &code.inst {
            Inst::Leaf(l, drive) => self.leaf(l, *drive, keys, &mut emit),
            Inst::Filter(e, s) => self.visit(&e.code, &args(e, keys, &[]), &mut |ctx, r| {
                ctx.project(s, r, &[], &mut emit)
            }),
            Inst::Join(first, other, side, s, semi) => {
                self.visit(&first.code, &args(first, keys, &[]), &mut |ctx, row| {
                    let params = args(other, keys, row);
                    let mut stop = false;
                    ctx.visit(&other.code, &params, &mut |ctx, b| {
                        let (l, r) = if *side == 0 { (row, b) } else { (b, row) };
                        ctx.project(s, l, r, &mut |ctx, out| {
                            stop = emit(ctx, out)?;
                            Ok(stop || *semi)
                        })
                    })?;
                    Ok(stop)
                })
            }
            Inst::Union(l, r) => {
                Ok(self.visit(l, keys, &mut emit)? || self.visit(r, keys, &mut emit)?)
            }
            Inst::Anti(l, r, ks) => self.visit(l, keys, &mut |ctx, row| {
                let params: Row = ks.iter().map(|&k| row[k]).collect();
                if ctx.visit(r, &params, &mut |_, _| Ok(true))? {
                    Ok(false)
                } else {
                    emit(ctx, row)
                }
            }),
            Inst::Transform(e, t) => self.visit(&e.code, &args(e, keys, &[]), &mut |ctx, r| {
                let mut out = Row::from_slice(r);
                match t {
                    Transform::Columns(cs) => {
                        out.clear();
                        out.extend(cs.iter().map(|&c| r[c]));
                    }
                    Transform::Append(v) => out.extend_from_slice(v),
                    Transform::Bounds(c) => {
                        out.push(r[*c] >> 32);
                        out.push(r[*c] & 0xffff_ffff);
                    }
                    Transform::Span(s, e) => out.push(if r[*s] == NULL || r[*e] == NULL {
                        NULL
                    } else {
                        r[*s] & 0xffff_ffff_0000_0000 | r[*e] & 0xffff_ffff
                    }),
                }
                emit(ctx, &out)
            }),
            Inst::Axis(a) => self.axis(a, keys, &mut emit),
            Inst::Predecessor(p) => {
                self.visit(&p.input.code, &args(&p.input, keys, &[]), &mut |ctx, r| {
                    let t = ctx.table(p.index.0, None)?;
                    let ix = if let Some(ix) = ctx.file.ordered.get(&p.index) {
                        ix.clone()
                    } else {
                        let start = clock();
                        let mut ix = OrderedPostings::default();
                        for j in 0..t.n {
                            let key = p.index.2.iter().map(|&c| t.cols[c][j]).collect();
                            ix.entry(key)
                                .or_default()
                                .push((t.cols[p.index.1][j] as i64, j));
                        }
                        for posting in ix.values_mut() {
                            posting.sort_unstable();
                        }
                        ctx.stat.builds += 1;
                        ctx.stat.rows += t.n as u64;
                        ctx.stat.index += clock() - start;
                        let ix = Rc::new(ix);
                        ctx.file.ordered.insert(p.index.clone(), ix.clone());
                        ix
                    };
                    ctx.stat.probes += 1;
                    let key: Row = p.partition.iter().map(|&(lc, _)| r[lc]).collect();
                    if let Some(posting) = ix.get(&key) {
                        let end = posting.partition_point(|&(v, _)| v <= r[p.left_order] as i64);
                        if end > 0 {
                            let latest = posting[end - 1].0;
                            let begin = posting[..end].partition_point(|&(v, _)| v < latest);
                            for &(_, j) in &posting[begin..end] {
                                let out: Row = p
                                    .project
                                    .iter()
                                    .map(|&(l, c)| if l { r[c] } else { t.cols[c][j] })
                                    .collect();
                                if emit(ctx, &out)? {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                    Ok(false)
                })
            }
        }
    }
    fn project(
        &mut self,
        s: &Select,
        l: &[u64],
        r: &[u64],
        emit: &mut dyn FnMut(&mut Self, &[u64]) -> R<bool>,
    ) -> R<bool> {
        self.stat.tests += s.tests.len() as u64;
        if s.tests.iter().all(|f| truthy(f(self.run, l, r))) {
            let row: Row = s.items.iter().map(|c| c.get(self.run, l, r)).collect();
            emit(self, &row)
        } else {
            Ok(false)
        }
    }
    fn axis(
        &mut self,
        a: &Axis,
        keys: &[u64],
        emit: &mut dyn FnMut(&mut Self, &[u64]) -> R<bool>,
    ) -> R<bool> {
        self.visit(&a.side.code, &args(&a.side, keys, &[]), &mut |ctx, r| {
            let first = r[if a.inverse { a.domain_id } else { a.id }];
            let up = matches!(
                a.dir,
                Direction::Up | Direction::Parent | Direction::Nearest
            ) != a.inverse;
            let one = matches!(a.dir, Direction::Child | Direction::Parent);
            let mut stack: SmallVec<[u64; 8]> = smallvec::smallvec![first];
            let mut seen = FxHashSet::default();
            seen.insert(first);
            while let Some(at) = stack.pop() {
                let mut neighbors: SmallVec<[u64; 8]> = SmallVec::new();
                ctx.visit(if up { &a.up } else { &a.down }, &[at], &mut |_, e| {
                    neighbors.push(e[if up { a.parent } else { a.child }]);
                    Ok(false)
                })?;
                for x in neighbors {
                    if !seen.insert(x) {
                        continue;
                    }
                    let mut accepted = false;
                    let stop = ctx.visit(&a.other, &[x], &mut |ctx, other| {
                        accepted = true;
                        let mut row = Row::from_slice(if a.inverse { other } else { r });
                        row.push(if a.inverse { first } else { x });
                        emit(ctx, &row)
                    })?;
                    if stop {
                        return Ok(true);
                    }
                    if matches!(a.dir, Direction::Nearest) {
                        if !a.inverse && accepted {
                            return Ok(false);
                        }
                        if a.inverse && ctx.visit(&a.domain_check, &[x], &mut |_, _| Ok(true))? {
                            continue;
                        }
                    }
                    if !one {
                        stack.push(x)
                    }
                }
            }
            Ok(false)
        })
    }
}
