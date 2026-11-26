//! Append each portable word to its input row, preserving input multiplicity.
use super::{NULL, R, Run, Table, Ty};
use ryo_values::domain::words::{
    service::split,
    types::{WordOutput, WordScratch},
};
use ryo_values::domain::row_texts::types::CellText;
struct Words(Vec<Box<str>>);
impl WordOutput for Words {
    fn word(&mut self, text: CellText<'_>) {
        self.0.push(text.0.into());
    }
}
pub(super) fn write(run: &mut Run, from: &Table, col: usize, out: &mut Table) -> R<()> {
    let mut emitted = Words(Vec::new());
    let mut scratch = String::new();
    let mut row = Vec::new();
    for i in 0..from.n {
        let cell = from.cols[col][i];
        if cell == NULL {
            continue;
        }
        let bytes = match from.tys[col] {
            Ty::File => run.paths.get(cell as usize).ok_or("unbound words file")?.as_bytes(),
            Ty::Sym => run.text(cell as u32),
            _ => continue,
        };
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        split(CellText(text), WordScratch(&mut scratch), &mut emitted);
        from.row(i, &mut row);
        for w in emitted.0.drain(..) {
            row.push(u64::from(run.sym(w.as_bytes())));
            out.push_row(&row);
            row.pop();
        }
    }
    Ok(())
}
