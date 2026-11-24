//! A kept finding shares its prepared column layout. The external comparison
//! writer borrows the same typed cells the product consumes.
use super::{Exec, FxHashMap, NULL, Names, Rc, Run, Table, Ty, Tys};
use ryo_relations::domain::relations::types::{Integer, Offset, Span};
use ryo_values::domain::row_texts::types::Cell;
use std::{cell::RefCell, collections::hash_map::Entry};

pub(crate) type Value = Option<Cell<Rc<str>>>;

/// Only strings named by a retained answer outlive the input file.
#[derive(Default)]
pub(super) struct Words {
    symbols: FxHashMap<u32, Rc<str>>,
    paths: Vec<Rc<str>>,
    path_ids: FxHashMap<Rc<str>, u32>,
}

pub(crate) struct Layout {
    pub rule: Box<str>,
    pub names: Names,
    pub identity: Box<[usize]>,
    types: Tys,
    words: Rc<RefCell<Words>>,
}

impl Layout {
    fn new(
        run: &mut Run,
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
            types: table.tys.clone(),
            words: run
                .answer_words
                .get_or_insert_with(Default::default)
                .clone(),
        }))
    }
}

impl Exec<'_> {
    pub(super) fn answer_layout(
        &mut self,
        run: &mut Run,
        emit: (usize, usize),
        rule: &str,
        table: &Table,
        subject: &str,
        keys: &[String],
    ) -> Result<Rc<Layout>, String> {
        Ok(match self.answer_layouts.entry(emit) {
            Entry::Occupied(slot) => slot.get().clone(),
            Entry::Vacant(slot) => slot
                .insert(Layout::new(run, rule, table, subject, keys)?)
                .clone(),
        })
    }
}

/// The schema gives each word its type. Node words retain their file and position.
#[derive(Clone)]
pub(crate) struct Row {
    pub layout: Rc<Layout>,
    words: Box<[u64]>,
}

impl Row {
    fn decode<T: From<String> + From<&'static str>>(
        &self,
        index: usize,
        dictionary: &Words,
        text: impl FnOnce(&Rc<str>) -> T,
    ) -> Option<Cell<T>> {
        let word = self.words[index];
        if word == NULL && self.layout.types[index] != Ty::Num {
            return None;
        }
        Some(match self.layout.types[index] {
            Ty::Sym => Cell::Text(text(&dictionary.symbols[&(word as u32)])),
            Ty::Node => {
                let path = &dictionary.paths[(word >> 32) as usize];
                Cell::Text(format!("{path}#{}", word as u32).into())
            }
            Ty::File => Cell::File(text(&dictionary.paths[word as usize])),
            Ty::Span => Cell::Span(Span {
                start: Offset((word >> 32) as u32),
                end: Offset(word as u32),
            }),
            Ty::Num => Cell::Number(Integer(word as i64)),
            Ty::Flag => Cell::Text((if word == 0 { "false" } else { "true" }).into()),
        })
    }

    pub(crate) fn value(&self, index: usize) -> Value {
        self.decode(index, &self.layout.words.borrow(), Rc::clone)
    }

    /// A caller owning its strings can take formatted node and flag text directly.
    pub(crate) fn owned_cells(&self) -> impl Iterator<Item = Option<Cell>> + '_ {
        let dictionary = self.layout.words.borrow();
        (0..self.words.len()).map(move |i| {
            self.decode(i, &dictionary, |s| Box::<str>::from(s.as_ref()))
        })
    }

    pub(crate) fn cells(&self) -> impl Iterator<Item = Value> + '_ {
        (0..self.words.len()).map(|i| self.value(i))
    }
}

impl Words {
    fn symbol(&mut self, run: &Run, id: u32) -> Result<&Rc<str>, String> {
        Ok(match self.symbols.entry(id) {
            Entry::Occupied(slot) => slot.into_mut(),
            Entry::Vacant(slot) => {
                let text = std::str::from_utf8(run.text(id)).map_err(|e| e.to_string())?;
                slot.insert(text.into())
            }
        })
    }

    // Project bindings can number a new inventory from zero in the same run.
    // Retained node and file cells name the path from the producing inventory.
    fn path(&mut self, text: &str) -> Result<u32, String> {
        if let Some(&id) = self.path_ids.get(text) {
            return Ok(id);
        }
        let id = u32::try_from(self.paths.len()).map_err(|e| e.to_string())?;
        let path: Rc<str> = text.into();
        self.paths.push(path.clone());
        self.path_ids.insert(path, id);
        Ok(id)
    }
}

pub(crate) fn typed(
    run: &Run,
    table: &Table,
    row: usize,
    file: Option<usize>,
    layout: &Rc<Layout>,
) -> Result<Row, String> {
    let mut dictionary = layout.words.borrow_mut();
    let mut values = Vec::with_capacity(table.cols.len());
    for (column, ty) in table.cols.iter().zip(layout.types.iter()) {
        let mut word = column[row];
        if word != NULL {
            match ty {
                Ty::Node => {
                    let file = match file {
                        Some(id) => u32::try_from(id).map_err(|e| e.to_string())?,
                        None => (word >> 32) as u32,
                    };
                    let path = run
                        .paths
                        .get(file as usize)
                        .ok_or("node has no source origin")?;
                    let origin = dictionary.path(path)?;
                    word = (u64::from(origin) << 32) | u64::from(word as u32);
                }
                Ty::File => {
                    let file = usize::try_from(word).map_err(|e| e.to_string())?;
                    let path = run
                        .paths
                        .get(file)
                        .ok_or_else(|| format!("file outside the run: {word}"))?;
                    let text = match run.path_sym.get(file) {
                        Some(id) => {
                            std::str::from_utf8(run.text(*id)).map_err(|e| e.to_string())?
                        }
                        None => path.as_str(),
                    };
                    word = u64::from(dictionary.path(text)?);
                }
                Ty::Sym if run.absent_word == Some(word as u32) => word = NULL,
                Ty::Sym => {
                    dictionary.symbol(run, word as u32)?;
                }
                Ty::Num | Ty::Span | Ty::Flag => {}
            }
        }
        values.push(word);
    }
    Ok(Row {
        layout: layout.clone(),
        words: values.into_boxed_slice(),
    })
}
