//! Consumers of an ordered row stream can read the provider columns in place.
//! A later consumer can still request a full projection of the same logical result.
use super::{
    Exec, Extent, File, Native, Provider, R, Run, Schema, Table, ViewSpec, cold_bucket::Rows, leaf_scan,
    pos, view_cols, view_rows};
use rustc_hash::FxHashMap;
use std::rc::Rc;

pub(crate) struct Cursor<'a> {
    pub(crate) view: Rc<ViewSpec>,
    pub rows: Rows<'a>,
    pub(crate) schema: Rc<Schema>,
}

pub(crate) enum Edges<'a> {
    Table(Rc<Table>),
    Cursor(Cursor<'a>),
}

pub(crate) type Adjacency = FxHashMap<Vec<u64>, Vec<u64>>;

impl Exec<'_> {
    pub(crate) fn cursor<'a>(
        &mut self,
        f: &mut File<'a>,
        run: &mut Run,
        p: &mut dyn Provider,
        node: usize,
        accepts: impl FnOnce(&ViewSpec) -> bool,
    ) -> R<Option<Cursor<'a>>> {
        if f.vmat.contains_key(&node) {
            return Ok(None);
        }
        let Some(view) = self.view(run, node)? else {
            return Ok(None);
        };
        // Gathering a tree text column may assign fresh run symbols. Keep the
        // original column order wherever that side effect could be observable.
        if (view.base.is_tree() && view.map.contains(&2)) || !accepts(&view) {
            return Ok(None);
        }
        if !matches!(self.res(f, run, p, node)?, super::Res::View(_)) {
            return Ok(None);
        }
        let t0 = leaf_scan::table_clock();
        let schema = self.schema(run, node)?;
        let rows = view_rows(f, run, &view, node);
        let cursor = Cursor { view, rows, schema };
        if f.cursor_views.insert(node) {
            self.stats.total_runs += 1;
            self.stats.total_rows += cursor.rows.len() as u64;
            let dt = leaf_scan::table_clock() - t0;
            if self.timing || true {
                self.inner += dt;
            }
        }
        Ok(Some(cursor))
    }

    pub(crate) fn edge_input<'a>(
        &mut self,
        f: &mut File<'a>,
        run: &mut Run,
        p: &mut dyn Provider,
        node: usize,
    ) -> R<Edges<'a>> {
        match self.cursor(f, run, p, node, |_| true)? {
            Some(cursor) => Ok(Edges::Cursor(cursor)),
            None => Ok(Edges::Table(self.eval(f, run, p, node)?)),
        }
    }

    pub(crate) fn file_exclusion(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        input: usize,
        keys: (&[usize], &[usize]),
        left: &Rc<Table>,
        gather: impl FnOnce(&[u32]) -> Rc<Table>,
    ) -> R<Option<Rc<Table>>> {
        let ([lc], [rc]) = keys else { return Ok(None) };
        let Some(cursor) = self.cursor(f, run, p, input, |v| {
            matches!(
                (v.base, v.map[*rc]),
                (Native::Tree(_), 3) | (Native::Child(_), 4) | (Native::Location, 1) | (Native::Token, 3)
            )
        })?
        else {
            return Ok(None);
        };
        if cursor.rows.is_empty() {
            return Ok(Some(left.clone()));
        }
        let mut keep = self.take_u32();
        keep.extend(
            (0..left.n as u32).filter(|i| left.cols[*lc][*i as usize] != u64::from(f.file)),
        );
        let result = gather(&keep);
        self.give_u32(keep);
        Ok(Some(result))
    }
}

impl Edges<'_> {
    pub(crate) fn col(&self, name: &str) -> R<usize> {
        match self {
            Self::Table(t) => t.col(name),
            Self::Cursor(c) => pos(&c.schema.names, name),
        }
    }

    pub(crate) fn adjacency(
        &self,
        f: &File<'_>,
        run: &mut Run,
        keys: &[usize],
        src: usize,
        dst: usize,
    ) -> (Adjacency, Adjacency) {
        match self {
            Self::Table(t) => maps(0..t.n, |r, c| t.cols[c][r], keys, src, dst),
            Self::Cursor(c) => maps(
                c.rows.iter().copied(),
                |r, col| view_cols::cell(f, run, c.view.base, r, c.view.map[col]),
                keys,
                src,
                dst,
            ),
        }
    }
}

pub(crate) fn maps<T: Copy>(
    rows: impl Iterator<Item = T>,
    mut cell: impl FnMut(T, usize) -> u64,
    keys: &[usize],
    src: usize,
    dst: usize,
) -> (Adjacency, Adjacency) {
    let mut adj = Adjacency::default();
    let mut reverse = Adjacency::default();
    for row in rows {
        let mut forward: Vec<u64> = keys.iter().map(|c| cell(row, *c)).collect();
        let mut back = forward.clone();
        let (s, d) = (cell(row, src), cell(row, dst));
        forward.push(s);
        adj.entry(forward).or_default().push(d);
        back.push(d);
        reverse.entry(back).or_default().push(s);
    }
    (adj, reverse)
}
