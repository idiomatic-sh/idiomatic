//! A kept finding shares its prepared column layout. The external comparison
//! writer borrows the same typed cells the product consumes.
use super::{Exec, FxHashMap, NULL, Run, Table, Ty};
use ryo_relations::domain::relations::types::{Integer, Offset, Span};
use ryo_values::domain::row_texts::types::Cell;
use std::{
    collections::hash_map::Entry,
    sync::{Arc, RwLock},
};

pub(crate) type Value = Option<Cell<Arc<str>>>;

/// Only strings named by a retained answer outlive the input file.
#[derive(Default)]
pub(crate) struct Words {
    symbols: FxHashMap<u32, Arc<str>>,
    paths: Vec<Arc<str>>,
    path_ids: FxHashMap<Arc<str>, u32>,
}

pub(crate) struct Layout {
    pub rule: Box<str>,
    pub names: Box<[String]>,
    pub identity: Box<[usize]>,
    types: Box<[Ty]>,
    words: Arc<RwLock<Words>>,
}

impl Layout {
    fn new(
        run: &mut Run,
        rule: &str,
        table: &Table,
        subject: &str,
        keys: &[String],
    ) -> Result<Arc<Self>, String> {
        let identity = std::iter::once(subject)
            .chain(keys.iter().map(String::as_str))
            .map(|name| table.col(name))
            .collect::<Result<_, _>>()?;
        Ok(Arc::new(Self {
            rule: rule.into(),
            names: table.names.as_slice().into(),
            identity,
            types: table.tys.as_slice().into(),
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
    ) -> Result<Arc<Layout>, String> {
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
    pub layout: Arc<Layout>,
    words: Box<[u64]>,
}

impl Row {
    /// Rebuild a row from the stable cell values at the retention boundary.
    #[allow(dead_code, reason = "the check session restores packed answers from persistent storage")]
    pub(crate) fn restored(
        rule: Box<str>,
        identity: Box<[usize]>,
        columns: Vec<(String, Value)>,
    ) -> Self {
        let mut dictionary = Words::default();
        let mut names = Vec::with_capacity(columns.len());
        let mut types = Vec::with_capacity(columns.len());
        let mut words = Vec::with_capacity(columns.len());
        for (name, cell) in columns {
            names.push(name);
            let (ty, word) = match cell {
                None => (Ty::Sym, NULL),
                Some(Cell::Number(n)) => (Ty::Num, n.0 as u64),
                Some(Cell::Span(s)) => {
                    (Ty::Span, (u64::from(s.start.0) << 32) | u64::from(s.end.0))
                }
                Some(Cell::File(path)) => {
                    let id = dictionary.paths.len() as u64;
                    dictionary.paths.push(path);
                    (Ty::File, id)
                }
                Some(Cell::Text(text)) => {
                    let id = u32::try_from(dictionary.symbols.len()).expect("stored row width");
                    dictionary.symbols.insert(id, text);
                    (Ty::Sym, u64::from(id))
                }
            };
            types.push(ty);
            words.push(word);
        }
        Self {
            layout: Arc::new(Layout {
                rule,
                names: names.into(),
                identity,
                types: types.into(),
                words: Arc::new(RwLock::new(dictionary)),
            }),
            words: words.into(),
        }
    }

    pub(crate) fn value(&self, index: usize) -> Value {
        let word = self.words[index];
        if word == NULL && self.layout.types[index] != Ty::Num {
            return None;
        }
        Some(match self.layout.types[index] {
            Ty::Sym => Cell::Text(
                self.layout.words.read().expect("answer words").symbols[&(word as u32)].clone(),
            ),
            Ty::Node => {
                let words = self.layout.words.read().expect("answer words");
                let path = &words.paths[(word >> 32) as usize];
                Cell::Text(format!("{path}#{}", word as u32).into())
            }
            Ty::File => Cell::File(
                self.layout.words.read().expect("answer words").paths[word as usize].clone(),
            ),
            Ty::Span => Cell::Span(Span {
                start: Offset((word >> 32) as u32),
                end: Offset(word as u32),
            }),
            Ty::Num => Cell::Number(Integer(word as i64)),
            Ty::Flag => Cell::Text((if word == 0 { "false" } else { "true" }).into()),
        })
    }

    #[cfg(feature = "product-session")]
    pub(crate) fn dictionary(&self) -> &RwLock<Words> {
        &self.layout.words
    }

    #[cfg(feature = "product-session")]
    pub(crate) fn value_from(&self, index: usize, words: &Words) -> Value {
        let word = self.words[index];
        let ty = self.layout.types[index];
        if word == NULL && ty != Ty::Num {
            return None;
        }
        match ty {
            Ty::Sym => Some(Cell::Text(words.symbols[&(word as u32)].clone())),
            Ty::Node => {
                let path = &words.paths[(word >> 32) as usize];
                Some(Cell::Text(format!("{path}#{}", word as u32).into()))
            }
            Ty::File => Some(Cell::File(words.paths[word as usize].clone())),
            _ => self.value(index),
        }
    }

    #[cfg(feature = "product-session")]
    pub(crate) fn width(&self) -> usize {
        self.words.len()
    }

    pub(crate) fn cells(&self) -> impl Iterator<Item = Value> + '_ {
        (0..self.words.len()).map(|i| self.value(i))
    }
}

impl Words {
    fn symbol(&mut self, run: &Run, id: u32) -> Result<&Arc<str>, String> {
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
        let path: Arc<str> = text.into();
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
    layout: &Arc<Layout>,
) -> Result<Row, String> {
    let mut dictionary = layout.words.write().expect("answer words");
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
                    word = u64::from(dictionary.path(path)?);
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
