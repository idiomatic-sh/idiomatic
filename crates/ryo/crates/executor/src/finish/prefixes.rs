//! THROWAWAY (redo-binding, ryo-fmx3.1): `prefixes`, the prefixes of a text that end at a separator.
//!
//! `prefixes(from=R, text="c", separator=".", output="prefix")`: every row of `R` once for each prefix `p` of its text column
//! `c` with `p = c` or `starts_with(c, concat(p, separator))`, with `p` in the new text column `output`. A join on `output`
//! then answers "this dotted name has that dotted name as a prefix" by key, where the condition
//! `l.c = r.p or starts_with(l.c, concat(r.p, "."))` had to meet every row of `r`. A row whose text is null makes no row.
//! No rule id: the text column and the separator are the arguments.

use super::{Exec, File, NULL, Provider, R, Run, Table, s};

impl Exec<'_> {
    pub fn prefixes(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {
        let a = self.plan.nodes[node].args();
        let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
        let tc = from.col(s(&a["text"])?)?;
        let sep = s(&a["separator"])?.as_bytes().to_vec();
        let _ = s(&a["output"])?;
        if sep.is_empty() {
            return Err("prefixes: an empty separator".to_owned().into());
        }
        let mut out = self.empty(run, node)?;
        let mut row = Vec::with_capacity(from.cols.len() + 1);
        for i in 0..from.n {
            let v = from.cols[tc][i];
            if v == NULL {
                continue;
            }
            let text = run.text(v as u32).to_vec();
            let mut cuts: Vec<usize> = (0..text.len()).filter(|k| text[*k..].starts_with(&sep)).collect();
            cuts.push(text.len());
            cuts.dedup();
            for k in cuts {
                row.clear();
                row.extend(from.cols.iter().map(|c| c[i]));
                row.push(u64::from(run.sym(&text[..k])));
                out.push_row(&row);
            }
        }
        Ok(out)
    }
}
