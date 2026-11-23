//! Consumers of an ordered row stream can read the provider columns in place.
//! A later consumer can still request a full projection of the same logical result.
use super::{
    Exec, File, Native, Provider, R, Run, Schema, Table, ViewSpec, cold_bucket::Rows, leaf_scan,
    pos, view_cols, view_rows_for,
};
use crate::finish::TableAccess as _;
use rustc_hash::FxHashMap;
use std::rc::Rc;

pub(super) struct Cursor<'a> {
    view: Rc<ViewSpec>,
    rows: Rows<'a>,
    schema: Rc<Schema>,
}

pub(super) enum Edges<'a> {
    Table(Rc<Table>),
    Cursor(Cursor<'a>),
}

type Adjacency = FxHashMap<Vec<u64>, Vec<u64>>;

impl Exec<'_> {
    fn cursor<'a>(
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
        if (view.base == Native::Tree && view.map.contains(&2)) || !accepts(&view) {
            return Ok(None);
        }
        if !matches!(self.res(f, run, p, node)?, super::Res::View(_)) {
            return Ok(None);
        }
        let t0 = leaf_scan::table_clock();
        let schema = self.schema(run, node)?;
        let rows = view_rows_for(f, run, &view, node, true);
        let cursor = Cursor { view, rows, schema };
        if f.cursor_views.insert(node) {
            self.stats.ran(node);
            self.stats.produced(node, cursor.rows.len());
            self.stats.built[node] += cursor.rows.len() as u64;
            let dt = leaf_scan::table_clock() - t0;
            self.stats.time[node] += dt;
            if self.timing || !sw!("VEC_LEAF_CLOCK") {
                self.inner += dt;
            }
        }
        Ok(Some(cursor))
    }

    pub(super) fn edge_input<'a>(
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

    pub(super) fn file_exclusion(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        node: usize,
        input: usize,
        keys: (&[usize], &[usize]),
        left: &Rc<Table>,
        gather: impl FnOnce(&[u32]) -> Rc<Table>,
    ) -> R<Option<Rc<Table>>> {
        let ([lc], [rc]) = keys else { return Ok(None) };
        let Some(cursor) = self.cursor(f, run, p, input, |v| {
            matches!(
                (v.base, v.map[*rc]),
                (Native::Tree, 3) | (Native::Child, 4) | (Native::Location, 1) | (Native::Token, 3)
            )
        })?
        else {
            return Ok(None);
        };
        self.stats.touched[node] += cursor.rows.len() as u64;
        if cursor.rows.is_empty() {
            return Ok(Some(left.clone()));
        }
        let mut keep = self.take_u32();
        keep.extend(
            (0..left.rows.0 as u32).filter(|i| left.cols[*lc][*i as usize] != u64::from(f.file)),
        );
        let result = gather(&keep);
        self.give_u32(keep);
        Ok(Some(result))
    }
}

impl Edges<'_> {
    pub(super) fn col(&self, name: &str) -> R<usize> {
        match self {
            Self::Table(t) => t.col(name),
            Self::Cursor(c) => pos(&c.schema.names, name),
        }
    }

    pub(super) fn adjacency(
        &self,
        f: &File<'_>,
        run: &mut Run,
        keys: &[usize],
        src: usize,
        dst: usize,
    ) -> (Adjacency, Adjacency) {
        match self {
            Self::Table(t) => maps(0..t.rows.0, |r, c| t.cols[c][r], keys, src, dst),
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

fn maps<T: Copy>(
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
