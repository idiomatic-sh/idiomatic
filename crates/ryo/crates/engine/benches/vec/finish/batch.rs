//! A mask is a selection of version partitions in one reader batch. Native
//! postings and node IDs are local to a partition. Each output keeps its tag
//! implicitly in its slot until the ordinary file sink consumes it.
use super::*;
use smallvec::SmallVec;

/// Sorted physical nodes reached by at least one file. Scratch survives batches;
/// unrelated plan slots are neither cleared nor visited.
#[derive(Default)]
pub(in super::super) struct Demand {
    masks: Vec<u32>,
    whole: Vec<u32>,
    pending: Vec<u64>,
    highest: usize,
    ordered: Vec<(usize, u32, u32)>,
}
impl Demand {
    pub(in super::super) fn include(&mut self, node: usize, files: u32) {
        self.select(node, files, true);
    }
    fn pop(&mut self) -> Option<usize> {
        loop {
            let word = self.pending.get_mut(self.highest)?;
            if *word != 0 {
                let bit = 63 - word.leading_zeros() as usize;
                *word ^= 1_u64 << bit;
                return Some(64 * self.highest + bit);
            }
            if self.highest == 0 {
                return None;
            }
            self.highest -= 1;
        }
    }
    fn select(&mut self, node: usize, files: u32, whole: bool) {
        if self.masks.len() <= node {
            self.whole.resize(node + 1, 0);
            self.masks.resize(node + 1, 0);
        }
        if self.masks[node] == 0 {
            let word = node / 64;
            if self.pending.len() <= word {
                self.pending.resize(word + 1, 0);
            }
            self.pending[word] |= 1_u64 << (node % 64);
            self.highest = self.highest.max(word);
        }
        self.masks[node] |= files;
        if whole {
            self.whole[node] |= files;
        }
    }
}

pub(in super::super) struct Lane<'a, 'v> {
    pub frame: &'a mut File<'v>,
    pub facts: &'a mut dyn Provider,
    pub fields: &'a mut Vec<u32>,
    pub root: &'a str,
}

impl Lane<'_, '_> {
    pub(super) fn enter<T>(
        &mut self,
        run: &mut Run,
        visit: impl FnOnce(&mut File<'_>, &mut Run, &mut dyn Provider) -> T,
    ) -> T {
        std::mem::swap(self.fields, &mut run.field_sym);
        if run.root_dir != self.root {
            run.root_dir.clear();
            run.root_dir.push_str(self.root);
        }
        let value = visit(self.frame, run, self.facts);
        std::mem::swap(self.fields, &mut run.field_sym);
        value
    }
}

impl Exec<'_> {
    pub(in super::super) fn prime_batch(
        &mut self,
        run: &mut Run,
        lanes: &mut [Lane<'_, '_>],
        demand: &mut Demand,
        roots: impl FnOnce(&mut Demand),
    ) -> bool {
        // With a single file there is no sibling to schedule alongside it.
        // Let the ordered sink demand each root, keeping provider refusals and
        // the attribution of step counts to their native rule intact.
        if lanes.len() <= 1 {
            return true;
        }
        roots(demand);
        // Descending node order collects the entire mask before visiting inputs.
        while let Some(node) = demand.pop() {
            let mut mask = std::mem::take(&mut demand.masks[node]);
            let whole = std::mem::take(&mut demand.whole[node]);
            if self.is_constant(run, node) {
                continue;
            }
            let step = &self.plan.nodes[node];
            // An indexed native view already represents its entire operator
            // tree. Only whole consumers need a gathered table for that view.
            if !matches!(self.view(run, node), Ok(Some(_))) {
                // An empty required result has no input demand. Check it before
                // distributing its mask, as the single-file producer does.
                if !sw!("VEC_NO_KNOWN") {
                    for (i, lane) in lanes.iter_mut().enumerate() {
                        let bit = 1 << i;
                        if mask & bit != 0
                            && lane.enter(run, |f, r, p| self.known_empty(f, r, p, node))
                        {
                            mask &= !bit;
                        }
                    }
                    if mask == 0 {
                        continue;
                    }
                }
                for &dep in &step.inputs {
                    let full = step.op == "union" || self.frames[node].gate as usize == dep;
                    let unary_source =
                        step.op == "select" && step.inputs.len() == 1 && step.semi().is_none();
                    // Keyed sources, the right side of a negative join and
                    // counted bands are asked for by their consumer as needed.
                    if full || unary_source {
                        demand.select(dep, mask, full);
                    }
                }
            }
            demand.ordered.push((node, mask, whole & mask));
        }
        let mut grouped = 0;
        let mut partitions = 0;
        while let Some((node, mask, whole)) = demand.ordered.pop() {
            let result = self.partition_step(run, lanes, node, mask, whole);
            match result {
                Ok(n) if run.missing.is_empty() => {
                    grouped += usize::from(n > 0);
                    partitions += n;
                }
                _ => {
                    run.missing.clear();
                    self.release_request();
                    demand.ordered.clear();
                    return false;
                }
            }
        }
        if sw!("VEC_BATCH_COUNT") {
            eprintln!(
                "BATCH files={} steps={} partitions={}",
                lanes.len(),
                grouped,
                partitions
            );
        }
        true
    }

    /// File ranges describe a single native relation. Bind its shape once and
    /// gather each requested column across the selected file ranges. Keep the
    /// native view for subsequent point queries on the same relation.
    fn materialize_partitions<'v>(
        &mut self,
        run: &mut Run,
        lanes: &mut [Lane<'_, 'v>],
        node: usize,
        mask: u32,
        view: &Rc<ViewSpec>,
    ) -> R<usize> {
        let schema = self.schema(run, node)?;
        let mut inputs = SmallVec::<[_; 4]>::new();
        let mut count = 0;
        for (i, lane) in lanes.iter_mut().enumerate() {
            if mask & (1 << i) == 0
                || lane.frame.vmat.contains_key(&node)
                || matches!(lane.frame.memo[node], Some(Res::Tab(_)))
            {
                continue;
            }
            // The returned positions borrow only the immutable file version.
            std::mem::swap(lane.fields, &mut run.field_sym);
            run.root_dir.clear();
            run.root_dir.push_str(lane.root);
            let load = self.load_fact(lane.frame, run, lane.facts, view.base);
            std::mem::swap(lane.fields, &mut run.field_sym);
            load?;
            lane.frame.remember(node, Res::View(view.clone()));
            std::mem::swap(lane.fields, &mut run.field_sym);
            let rows = view_rows_for(lane.frame, run, view, node, true);
            std::mem::swap(lane.fields, &mut run.field_sym);
            count += 1;
            if rows.is_empty() {
                let empty = self.empty_rc(run, node)?;
                if !lane.frame.cursor_views.remove(&node) {
                    self.stats.ran(node);
                }
                lane.frame.vmat.insert(node, empty);
            } else {
                let output = Table::with_rows(schema.names.clone(), schema.tys.clone(), rows.len());
                inputs.push((i, rows, output));
            }
        }
        for (col, &pick) in view.map.iter().enumerate() {
            for (id, rows, table) in &mut inputs {
                lanes[*id].enter(run, |f, r, _| {
                    view_cols::gather_ids(
                        f,
                        r,
                        view.base,
                        pick,
                        rows.iter().copied(),
                        &mut table.cols[col],
                    )
                });
            }
        }
        for (id, rows, mut output) in inputs {
            let f = &mut lanes[id].frame;
            output.n = rows.len();
            if !f.cursor_views.remove(&node) {
                self.stats.built[node] += output.n as u64;
                self.stats.ran(node);
                self.stats.produced(node, output.n);
            }
            let t = if output.n == 0 {
                self.empty_rc(run, node)?
            } else {
                recycle::rc(output)
            };
            f.vmat.insert(node, t);
        }
        Ok(count)
    }

    fn partition_step(
        &mut self,
        run: &mut Run,
        lanes: &mut [Lane<'_, '_>],
        node: usize,
        mask: u32,
        whole: u32,
    ) -> R<usize> {
        let plan = self.plan;
        let op = plan.nodes[node].op.as_str();
        if let Some(view) = self.view(run, node)? {
            for (i, lane) in lanes.iter_mut().enumerate() {
                if (mask & !whole) & (1 << i) != 0 && lane.frame.memo[node].is_none() {
                    // A later source read obtains the file's fact columns lazily.
                    lane.frame.remember(node, Res::View(view.clone()));
                }
            }
            if whole == 0 {
                return Ok(0);
            }
            return self.materialize_partitions(run, lanes, node, whole, &view);
        }
        let mut active = SmallVec::<[usize; 4]>::new();
        let empty = self.empty_rc(run, node)?;
        for (i, lane) in lanes.iter_mut().enumerate() {
            if mask & (1 << i) == 0 || lane.frame.memo[node].is_some() {
                continue;
            }
            active.push(i);
        }
        if active.is_empty() {
            return Ok(0);
        }
        let a = plan.nodes[node].args();
        let mut ready = SmallVec::<[(usize, Option<Rc<Table>>); 4]>::with_capacity(active.len());
        let first = self.frames[node].first;
        for id in active {
            let input = if self.frames[node].gate < frame::MISSING && op != "union" {
                Some(lanes[id].enter(run, |f, r, p| self.eval(f, r, p, first as usize))?)
            } else {
                None
            };
            if input.as_ref().is_some_and(|t| t.n == 0) {
                lanes[id].frame.remember(node, Res::Tab(empty.clone()));
            } else {
                ready.push((id, input));
            }
        }
        let parts = ready.len();
        if parts == 0 {
            return Ok(0);
        }
        let results: Vec<Rc<Table>> = match op {
            "capture" => {
                let input = ready[0].1.as_ref().unwrap();
                let path = input.col(s(&a["path"])?)?;
                let pattern = s(&a["pattern"])?;
                let slots = capture::names(pattern);
                let cols = strs(&a["captures"])?
                    .map(|c| slots.iter().position(|slot| slot == c))
                    .collect::<Vec<_>>();
                let mut output = ready
                    .iter()
                    .map(|_| self.empty(run, node))
                    .collect::<R<Vec<_>>>()?;
                capture::partitions(
                    run,
                    ready
                        .iter()
                        .zip(&mut output)
                        .map(|((_, t), out)| (t.as_deref().unwrap(), out)),
                    path,
                    pattern,
                    a.get("separator").and_then(Param::as_str),
                    &cols,
                    false,
                    &[],
                )?;
                output.into_iter().map(recycle::rc).collect()
            }
            "project" => {
                let input = ready[0].1.as_ref().unwrap();
                let cols = strs(&a["columns"])?
                    .map(|c| input.col(c))
                    .collect::<R<Vec<_>>>()?;
                let mut row = Vec::with_capacity(cols.len());
                let mut seen = rustc_hash::FxHashSet::default();
                let mut result = Vec::with_capacity(parts);
                for (_, part) in &ready {
                    let input = part.as_ref().unwrap();
                    let mut out = self.empty(run, node)?;
                    projected(input, &cols, &mut row, &mut seen, &mut out);
                    result.push(recycle::rc(out));
                }
                result
            }
            "select" => self.select_partitions(run, lanes, node, &ready)?,
            "axis" => {
                let rel = self.arg_rel(node, "relation")?;
                let direction = s(&a["direction"])?;
                ready
                    .iter()
                    .map(|(id, input)| {
                        lanes[*id].enter(run, |f, r, p| {
                            self.axis_partition(
                                f,
                                r,
                                p,
                                node,
                                a,
                                input.as_deref().unwrap(),
                                rel,
                                direction,
                            )
                            .map(recycle::rc)
                        })
                    })
                    .collect::<R<_>>()?
            }
            "union" => {
                let prepared = union_kernel::prep(self, run, node)?;
                ready
                    .iter()
                    .map(|(id, _)| {
                        lanes[*id].enter(run, |f, r, p| {
                            union_kernel::partition(self, f, r, p, &prepared)
                        })
                    })
                    .collect::<R<_>>()?
            }
            "antijoin" => {
                let prepared = antijoin_kernel::prep(self, run, node)?;
                ready
                    .iter()
                    .map(|(id, input)| {
                        lanes[*id].enter(run, |f, r, p| {
                            antijoin_kernel::over(
                                self,
                                f,
                                r,
                                p,
                                node,
                                &prepared,
                                input.as_ref().unwrap(),
                            )
                        })
                    })
                    .collect::<R<_>>()?
            }
            _ => ready
                .iter()
                .map(|(id, _)| lanes[*id].enter(run, |f, r, p| self.body(f, r, p, node)))
                .collect::<R<_>>()?,
        };
        for ((id, _), out) in ready.into_iter().zip(results) {
            self.stats.ran(node);
            self.stats.produced(node, out.n);
            if out.n == 0 {
                self.stats.empty[node] += 1;
            }
            keyed::built(self, lanes[id].frame, node, &out);
            lanes[id].frame.remember(node, Res::Tab(out));
        }
        Ok(parts)
    }
}

pub(super) fn projected(
    input: &Table,
    cols: &[usize],
    row: &mut Vec<u64>,
    seen: &mut rustc_hash::FxHashSet<inline_key::Key>,
    out: &mut Table,
) {
    seen.clear();
    for i in 0..input.n {
        row.clear();
        row.extend(cols.iter().map(|&c| input.cols[c][i]));
        if !seen.contains(&row[..]) {
            seen.insert(inline_key::Key::of(row));
            out.push_row(row);
        }
    }
}
