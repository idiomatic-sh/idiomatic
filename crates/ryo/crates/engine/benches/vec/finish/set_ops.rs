//! Whole-row set operations and grouped distinct-value digests.
use super::{Args, R, Run, Table, Ty, s, strs};
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use std::hash::Hasher;

pub(super) fn select_rows(left: &Table, right: &Table, intersection: bool) -> R<Table> {
    let order = left
        .names
        .iter()
        .map(|name| right.col(name))
        .collect::<R<Vec<_>>>()?;
    if order.len() != right.names.len()
        || order
            .iter()
            .enumerate()
            .any(|(i, &j)| left.tys[i] != right.tys[j])
    {
        return Err("set operands have different column shapes".into());
    }
    let members: FxHashSet<Vec<u64>> = (0..right.n)
        .map(|r| order.iter().map(|&c| right.cols[c][r]).collect())
        .collect();
    let mut out = Table::with(left.names.clone(), left.tys.clone());
    let mut seen = FxHashSet::default();
    let mut row = Vec::new();
    for i in 0..left.n {
        left.row(i, &mut row);
        if members.contains(&row) == intersection && seen.insert(row.clone()) {
            out.push_row(&row);
        }
    }
    Ok(out)
}

/// Mirror the shipping reduction: hash resolved values, sort and deduplicate the
/// hashes, then fold them in order. File and text cells hash their spelling.
pub(super) fn digest(run: &Run, file: u32, args: &Args, input: &Table, output: &mut Table) -> R<()> {
    let keys = strs(&args["keys"])?
        .iter()
        .map(|name| input.col(name))
        .collect::<R<Vec<_>>>()?;
    let distinct = input.col(s(&args["distinct"])?)?;
    let mut groups: FxHashMap<Vec<u64>, Vec<u64>> = FxHashMap::default();
    for row in 0..input.n {
        let key = keys.iter().map(|&c| input.cols[c][row]).collect();
        let value = input.cols[distinct][row];
        let mut hash = FxHasher::default();
        match input.tys[distinct] {
            Ty::Sym => hash.write(run.text(value as u32)),
            Ty::File => hash.write(
                run.paths
                    .get(value as usize)
                    .ok_or("unknown digest file")?
                    .as_bytes(),
            ),
            Ty::Flag => hash.write_u8(u8::from(value == 1)),
            Ty::Num => hash.write_i64(value as i64),
            Ty::Node => hash.write_u64(u64::from(file) << 32 | value),
            Ty::Span if value == super::NULL => {
                hash.write(ryo_relations::domain::relations::constants::UNLOCATED.as_bytes())
            }
            Ty::Span => hash.write_i64((value >> 32) as i64),
        }
        groups.entry(key).or_default().push(hash.finish());
    }
    for (mut row, mut hashes) in groups {
        hashes.sort_unstable();
        hashes.dedup();
        let mut result = FxHasher::default();
        for &hash in &hashes {
            result.write_u64(hash);
        }
        row.extend([result.finish(), hashes.len() as u64]);
        output.push_row(&row);
    }
    Ok(())
}
