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
pub(super) struct Words {
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

/// Rebuild one ordered packet using a shared dictionary and column layouts.
#[allow(dead_code, reason = "only the retained check session restores answer packets")]
pub(crate) struct RestoredRows {
    dictionary: Arc<RwLock<Words>>,
    layouts: Vec<FxHashMap<Vec<Ty>, Arc<Layout>>>,
}

#[allow(dead_code, reason = "file readers share the answer module without loading stored packets")]
impl RestoredRows {
    pub(crate) fn new(words: Vec<Box<str>>, schemas: usize) -> Self {
        let paths: Vec<Arc<str>> = words.into_iter().map(Arc::from).collect();
        let symbols = paths.iter().enumerate().map(|(i, s)| {
            (u32::try_from(i).expect("stored word dictionary"), s.clone())
        }).collect();
        Self {
            dictionary: Arc::new(RwLock::new(Words { paths, symbols, ..Default::default() })),
            layouts: (0..schemas).map(|_| FxHashMap::default()).collect(),
        }
    }

    pub(crate) fn row(&mut self, schema: usize, rule: &str, names: &[Box<str>], identity: &[usize], cells: impl Iterator<Item = (Ty, u64)>) -> Row {
        // Null in a numeric column must remain distinct from the number -1.
        let (types, words): (Vec<_>, Vec<_>) = cells.unzip();
        let layout = self.layouts[schema].entry(types).or_insert_with_key(|types| Arc::new(Layout {
            rule: rule.into(),
            names: names.iter().map(|n| n.to_string()).collect(),
            identity: identity.into(),
            types: types.as_slice().into(),
            words: self.dictionary.clone(),
        }));
        Row { layout: layout.clone(), words: words.into() }
    }
}

impl Row {
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
