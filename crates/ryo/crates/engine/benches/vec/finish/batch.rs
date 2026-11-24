//! A mask is a selection of version partitions in one reader batch. Native
//! postings and node IDs are local to a partition. Each output keeps its tag
//! implicitly in its slot until the ordinary file sink consumes it.
use super::*;
use std::collections::BTreeMap;

pub(in super::super) struct Lane<'a,'v> {
    pub frame: &'a mut File<'v>,
    pub facts: &'a mut dyn Provider,
    pub fields: &'a mut Vec<u32>,
    pub root: &'a str,
}

impl Lane<'_, '_> {
    fn enter<T>(&mut self, run: &mut Run, visit: impl FnOnce(&mut File<'_>, &mut Run, &mut dyn Provider) -> T) -> T {
        std::mem::swap(self.fields, &mut run.field_sym);
        run.root_dir.clear();
        run.root_dir.push_str(self.root);
        let value = visit(self.frame, run, self.facts);
        std::mem::swap(self.fields, &mut run.field_sym);
        value
    }
}

impl Exec<'_> {
    pub(in super::super) fn prime_batch(&mut self, run: &mut Run, lanes: &mut [Lane<'_, '_>], roots: BTreeMap<usize, Vec<usize>>) -> bool {
        if lanes.len() < 2 { return true; }
        let mut masks = BTreeMap::<usize,u32>::new();
        for (node, files) in roots {
            masks.insert(node,files.into_iter().fold(0,|m,f| m | (1 << f)));
        }
        let mut steps = Vec::new();
        // Whole unary inputs are unconditional. A selective join still gets
        // to choose its own posting and driver in each participating file.
        while let Some((node, mask)) = masks.pop_last() {
            let step = &self.plan.nodes[node];
            if step.op == "union" || matches!(step.op.as_str(), "project" | "capture" | "axis" | "extend" | "antijoin") {
                let inputs: &[usize] = if step.op == "union" { &step.inputs }
                    else if self.frames[node].first < frame::MISSING { std::slice::from_ref(&step.inputs[step.inputs.iter().position(|&n| n==self.frames[node].first as usize).unwrap()]) }
                    else { &[] };
                for &dep in inputs {
                    debug_assert!(dep < node);
                    *masks.entry(dep).or_default() |= mask;
                }
            }
            steps.push((node,mask));
        }
        let mut grouped = 0;
        let mut partitions = 0;
        for (node, mask) in steps.into_iter().rev() {
            if mask.count_ones()<2 || self.is_constant(run,node) { continue; }
            let result = self.partition_step(run,lanes,node,mask);
            match result {
                Ok(n) if run.missing.is_empty() => {
                    grouped+=usize::from(n>0);
                    partitions+=n;
                }
                _ => { run.missing.clear(); self.release_request(); return false; }
            }
        }
        if sw!("VEC_BATCH_COUNT") { eprintln!("BATCH files={} steps={} partitions={}",lanes.len(),grouped,partitions); }
        true
    }

    fn partition_step(&mut self, run: &mut Run, lanes: &mut [Lane<'_, '_>], node: usize, mask:u32) -> R<usize> {
        let plan = self.plan;
        let op = plan.nodes[node].op.as_str();
        if !matches!(op,"select"|"axis"|"capture"|"project"|"union"|"antijoin") || self.view(run,node)?.is_some() {
            return Ok(0);
        }
        let mut active=Vec::new();
        let empty = self.empty_rc(run,node)?;
        for (i,lane) in lanes.iter_mut().enumerate() {
            if mask & (1<<i)==0 || lane.frame.memo[node].is_some() { continue; }
            if !sw!("VEC_NO_KNOWN") && lane.enter(run,|f,r,p|self.known_empty(f,r,p,node)) {
                lane.frame.remember(node,Res::Tab(empty.clone()));
                continue;
            }
            active.push(i);
        }
        if active.is_empty() { return Ok(0); }
        let a = plan.nodes[node].args();
        let mut ready = Vec::with_capacity(active.len());
        let first = self.frames[node].first;
        for id in active {
            let input = if first < frame::MISSING && op != "union" {
                Some(lanes[id].enter(run,|f,r,p|self.eval(f,r,p,first as usize))?)
            } else { None };
            if input.as_ref().is_some_and(|t|t.n==0) {
                lanes[id].frame.remember(node,Res::Tab(empty.clone()));
            } else { ready.push((id,input)); }
        }
        let parts=ready.len();
        if parts==0 { return Ok(0); }
        let results: Vec<Rc<Table>> = match op {
            "capture" => {
                let input=ready[0].1.as_ref().unwrap();
                let path=input.col(s(&a["path"])?)?;
                let pattern=s(&a["pattern"])?;
                let slots=capture::names(pattern);
                let cols=strs(&a["captures"])?.map(|c|slots.iter().position(|slot|slot==c)).collect::<Vec<_>>();
                let mut output=ready.iter().map(|_| self.empty(run,node)).collect::<R<Vec<_>>>()?;
                capture::partitions(run,ready.iter().zip(&mut output).map(|((_,t),out)|(t.as_deref().unwrap(),out)),path,pattern,a.get("separator").and_then(Param::as_str),&cols,false,&[])?;
                output.into_iter().map(recycle::rc).collect()
            }
            "project" => {
                let input=ready[0].1.as_ref().unwrap();
                let cols=strs(&a["columns"])?.map(|c|input.col(c)).collect::<R<Vec<_>>>()?;
                let mut row=Vec::with_capacity(cols.len());
                let mut seen=rustc_hash::FxHashSet::default();
                let mut result=Vec::with_capacity(parts);
                for (_,part) in &ready {
                    let input=part.as_ref().unwrap();
                    let mut out=self.empty(run,node)?;
                    projected(input,&cols,&mut row,&mut seen,&mut out);
                    result.push(recycle::rc(out));
                }
                result
            }
            "select" => {
                let projection=self.projection(run,node);
                let mut result=Vec::with_capacity(parts);
                for &(id,_) in &ready {
                    result.push(lanes[id].enter(run,|f,r,p|match projection {
                        Some(index)=>self.col_select(f,r,p,node,index),
                        None=>self.select_entry(f,r,p,node),
                    })?);
                }
                result
            }
            _ => {
                let mut result=Vec::with_capacity(parts);
                for (id,input) in &ready {
                    result.push(lanes[*id].enter(run,|f,r,p|match op {
                        "axis"=>self.axis_over(f,r,p,node,a,input.as_deref().unwrap()).map(recycle::rc),
                        "union"=>union_kernel::whole(self,f,r,p,node),
                        _=>antijoin_kernel::whole(self,f,r,p,node),
                    })?);
                }
                result
            }
        };
        for ((id,_),out) in ready.into_iter().zip(results) {
            self.stats.ran(node);
            self.stats.produced(node,out.n);
            if out.n==0 {self.stats.empty[node]+=1;}
            keyed::built(self,lanes[id].frame,node,&out);
            lanes[id].frame.remember(node,Res::Tab(out));
        }
        Ok(parts)
    }
}

pub(super) fn projected(input:&Table, cols:&[usize], row:&mut Vec<u64>, seen:&mut rustc_hash::FxHashSet<inline_key::Key>, out:&mut Table) {
    seen.clear();
    for i in 0..input.n {
        row.clear();
        row.extend(cols.iter().map(|&c|input.cols[c][i]));
        if !seen.contains(&row[..]) {
            seen.insert(inline_key::Key::of(row));
            out.push_row(row);
        }
    }
}
