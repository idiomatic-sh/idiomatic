//! A single equality against an indexed base view has a known key on each side.
//! Read the already chosen driver in its original order and gather only the result.
use super::{
    E, Exec, File, Native, Op, R, Run, Src, Table, cold_bucket::Rows, recycle, view_cols,
    view_probe, view_rows,
};
use super::select_kernel::Pushed;
use std::rc::Rc;

pub fn columns(conj: &[E]) -> Option<[usize; 2]> {
    let [E::Cmp(Op::Eq, x, y)] = conj else {
        return None;
    };
    match (&**x, &**y) {
        (E::Col(0, a, _), E::Col(1, b, _)) | (E::Col(1, b, _), E::Col(0, a, _)) => Some([*a, *b]),
        _ => None,
    }
}

pub struct Projection<'a> {
    pub items: &'a [(usize, usize)],
    pub left: &'a mut Vec<u32>,
    pub right: &'a mut Vec<u32>,
    pub pushed: Option<&'a Pushed>,
}

pub fn run(
    ex: &mut Exec<'_>,
    f: &mut File<'_>,
    run: &mut Run,
    node: usize,
    sources: &[Src],
    driver: usize,
    columns: [usize; 2],
    out: Projection<'_>,
) -> R<Option<Rc<Table>>> {
    let other = 1 - driver;
    let Src::View(target, _) = &sources[other] else {
        return Ok(None);
    };
    let key = target.map[columns[other]];
    if !view_probe::supports(f, target, key) {
        return Ok(None);
    }
    let retained: Rows<'_>;
    let rows = match &sources[driver] {
        Src::Tab(t, _) => view_probe::Ids::Range(t.n as u32),
        Src::View(v, _)
            if v.filter.is_empty() && matches!(v.base, Native::Tree(_) | Native::Location) =>
        {
            view_probe::Ids::Range(f.limit(v.base.extent()) as u32)
        }
        Src::View(v, step) => {
            retained = view_rows(f, run, v, *step);
            view_probe::Ids::View(&retained)
        }
        _ => return Ok(None),
    };
    let n = rows.len();
    if n == 0 {
        return ex.empty_rc(run, node).map(Some);
    }
    out.left.clear();
    out.right.clear();
    let pushed = out.pushed.filter(|test| matches!(sources[test.source], Src::View(..)));
    if super::view_choice::reverse(
        f, run, sources, driver, columns, rows, pushed, out.left, out.right,
    ).is_none() {
        let batch = view_probe::Driver {
            rows,
            source: driver,
            column: columns[driver],
        };
        view_probe::filtered_pairs(f, run, sources, batch, target, key, pushed, |_, left, right| {
            out.left.push(left);
            out.right.push(right);
        })
        .expect("validated native view key");
    }
    if out.left.is_empty() {
        return ex.empty_rc(run, node).map(Some);
    }
    let mut table = ex.empty_rows(run, node, out.left.len())?;
    for ((source, column), dst) in out.items.iter().zip(&mut table.cols) {
        let positions = if *source == driver {
            &*out.left
        } else {
            &*out.right
        };
        match &sources[*source] {
            Src::Tab(t, _) => {
                let values = &t.cols[*column];
                dst.extend(positions.iter().map(|r| values[*r as usize]));
            }
            Src::View(v, _) => view_cols::gather(f, run, v.base, v.map[*column], positions, dst),
            _ => unreachable!("both sources resolved"),
        }
    }
    table.n = out.left.len();
    Ok(Some(recycle::rc(table)))
}
