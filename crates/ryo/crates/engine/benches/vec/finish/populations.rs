//! Kernels over numbered columns, valid for both a file and a closed project.
use super::{Args, NULL, R, Run, Table, Ty, s, strs};
use ryo_operators::domain::cycles::{
    service::components,
    types::{CycleMark, CycleScratch, NodeCount, NodeNumber},
};
use ryo_values::domain::{plan_parts::types::LiteralText, row_texts::types::CellText};

pub(super) fn cyclic(edges: &Table, source: usize, destination: usize, out: &mut Table) -> R<()> {
    if edges.tys[source] != edges.tys[destination] {
        return Err("cycle endpoints have different types".into());
    }
    let mut nodes = edges.cols[source].clone();
    nodes.extend_from_slice(&edges.cols[destination]);
    nodes.sort_unstable();
    nodes.dedup();
    let mut adjacency = vec![Vec::new(); nodes.len()];
    for (&a, &b) in edges.cols[source].iter().zip(&edges.cols[destination]) {
        let index = |n| nodes.binary_search(&n).expect("endpoint was numbered");
        adjacency[index(a)].push(NodeNumber(index(b)));
    }
    let mut scratch = CycleScratch::default();
    components(
        NodeCount(nodes.len()),
        |n, i| adjacency[n.0].get(i.0).copied(),
        &mut scratch,
    );
    for (node, present) in nodes.into_iter().zip(scratch.found) {
        if present == CycleMark::Raised {
            out.push_row(&[node, node]);
        }
    }
    Ok(())
}

pub(super) fn ranked(run: &Run, input: &Table, args: &Args, out: &mut Table) -> R<()> {
    let col = input.col(s(&args["column"])?)?;
    let layers = args["sets"]
        .as_array()
        .ok_or("rank needs bound sets of text")?
        .iter()
        .map(|set| Ok(strs(set)?.map(|t| LiteralText(t.into())).collect()))
        .collect::<R<Vec<Vec<LiteralText>>>>()?;
    let mut row = Vec::new();
    for i in 0..input.n {
        let word = input.cols[col][i];
        if word == NULL {
            continue;
        }
        let text = match input.tys[col] {
            Ty::File => run.paths.get(word as usize).map(String::as_str),
            Ty::Sym => std::str::from_utf8(run.text(word as u32)).ok(),
            _ => None,
        };
        if let Some(rank) =
            text.and_then(|t| ryo_operators::domain::rank::service::rank(CellText(t), &layers))
        {
            input.row(i, &mut row);
            row.push(rank.0 as u64);
            out.push_row(&row);
        }
    }
    Ok(())
}
