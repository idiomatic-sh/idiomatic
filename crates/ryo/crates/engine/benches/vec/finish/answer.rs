//! A kept finding shares its prepared column layout. The external comparison
//! writer borrows the same typed cells the product consumes.
use super::{Exec, NULL, Names, Rc, Run, Table, Ty};
use ryo_relations::domain::relations::types::{Integer, Offset, Span};
use ryo_values::domain::row_texts::types::Cell;

pub(crate) type Value = Option<Cell<Rc<str>>>;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Layout {
    pub rule: Box<str>,
    pub names: Names,
    pub identity: Box<[usize]>,
}

impl Layout {
    pub(super) fn new(
        rule: &str,
        table: &Table,
        subject: &str,
        keys: &[String],
    ) -> Result<Rc<Self>, String> {
        let identity = std::iter::once(subject)
            .chain(keys.iter().map(String::as_str))
            .map(|name| table.col(name))
            .collect::<Result<_, _>>()?;
        Ok(Rc::new(Self {
            rule: rule.into(),
            names: table.names.clone(),
            identity,
        }))
    }
}

impl Exec<'_> {
    pub(super) fn answer_layout(
        &mut self,
        emit: usize,
        rule: &str,
        table: &Table,
        subject: &str,
        keys: &[String],
    ) -> Result<Rc<Layout>, String> {
        use std::collections::hash_map::Entry;
        Ok(match self.answer_layouts.entry(emit) {
            Entry::Occupied(slot) => slot.get().clone(),
            Entry::Vacant(slot) => slot
                .insert(Layout::new(rule, table, subject, keys)?)
                .clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Row {
    pub layout: Rc<Layout>,
    pub values: Box<[Value]>,
}

fn symbol(run: &mut Run, id: u32) -> Result<Rc<str>, String> {
    if let Some(value) = run.answer_symbols.get(&id) {
        return Ok(value.clone());
    }
    let text = std::str::from_utf8(run.text(id)).map_err(|e| e.to_string())?;
    let value: Rc<str> = Rc::from(text);
    run.answer_symbols.insert(id, value.clone());
    Ok(value)
}

pub(crate) fn typed(
    run: &mut Run,
    table: &Table,
    row: usize,
    file: Option<usize>,
    layout: &Rc<Layout>,
) -> Result<Row, String> {
    let mut values = Vec::with_capacity(table.names.len());
    for (column, ty) in table.cols.iter().zip(table.tys.iter()) {
        let word = column[row];
        let cell = if word == NULL {
            None
        } else {
            match ty {
                Ty::Node => {
                    let (id, at) = match file {
                        Some(id) => (id, word),
                        None => ((word >> 32) as usize, word & u64::from(u32::MAX)),
                    };
                    let path = run.paths.get(id).ok_or("node has no source origin")?;
                    Some(Cell::Text(format!("{path}#{at}").into()))
                }
                Ty::File => {
                    let at = usize::try_from(word).map_err(|e| e.to_string())?;
                    let path = run
                        .paths
                        .get(at)
                        .ok_or_else(|| format!("file outside the run: {word}"))?;
                    let text = match run.path_sym.get(at) {
                        Some(&id) => symbol(run, id)?,
                        None => path.as_str().into(),
                    };
                    Some(Cell::File(text))
                }
                Ty::Sym if run.absent_word == Some(word as u32) => None,
                Ty::Sym => Some(Cell::Text(symbol(run, word as u32)?)),
                Ty::Span => Some(Cell::Span(Span {
                    start: Offset((word >> 32) as u32),
                    end: Offset(word as u32),
                })),
                Ty::Num => Some(Cell::Number(Integer(word as i64))),
                Ty::Flag => Some(Cell::Text(
                    (if word == 0 { "false" } else { "true" }).into(),
                )),
            }
        };
        values.push(cell);
    }
    Ok(Row {
        layout: layout.clone(),
        values: values.into_boxed_slice(),
    })
}
