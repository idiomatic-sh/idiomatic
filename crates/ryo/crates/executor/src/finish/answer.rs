//! A kept finding shares its prepared column layout. The external comparison
//! writer borrows the same typed cells the product consumes.
use super::{Exec, FxHashMap, NULL, Run, Table, Ty};
use ryo_relations::domain::relations::types::{Integer, Offset, Span};
use ryo_values::domain::row_texts::types::Cell;
use std::{
    collections::hash_map::Entry,
    sync::{Arc, RwLock},
};

pub mod report;
pub use report::Reported;

pub type Value = Option<Cell<Arc<str>>>;

/// Only strings named by a retained answer outlive the input file.
#[derive(Default)]
pub struct Words {
    pub symbols: FxHashMap<u32, Arc<str>>,
    pub paths: Vec<Arc<str>>,
    pub path_ids: FxHashMap<Arc<str>, u32>,
}

pub struct Layout {
    pub rule: Box<str>,
    pub names: Box<[String]>,
    pub identity: Box<[usize]>,
    pub types: Box<[Ty]>,
    pub words: Arc<RwLock<Words>>,
}

impl Layout {
    pub fn new(
        run: &mut Run,
        rule: &str,
        names: &[String],
        types: &[Ty],
        subject: &str,
        keys: &[String],
    ) -> Result<Arc<Self>, crate::check_session::refusal::Refusal> {
        let identity = std::iter::once(subject)
            .chain(keys.iter().map(String::as_str))
            .map(|name| names.iter().position(|column| column == name)
                 .ok_or_else(|| format!("no column {name} in {names:?}")))
            .collect::<Result<_, _>>()?;
        Ok(Arc::new(Self {
            rule: rule.into(),
            names: names.into(),
            identity,
            types: types.into(),
            words: run
                .answer_words
                .get_or_insert_with(Default::default)
                .clone(),
        }))
    }
}

impl Exec<'_> {
    pub fn answer_layout(
        &mut self,
        run: &mut Run,
        emit: (usize, usize),
        rule: &str,
        table: &Table,
        subject: &str,
        keys: &[String],
    ) -> Result<Arc<Layout>, crate::check_session::refusal::Refusal> {
        Ok(match self.answer_layouts.entry(emit) {
            Entry::Occupied(slot) => slot.get().clone(),
            Entry::Vacant(slot) => slot
                .insert(Layout::new(run, rule, &table.names, &table.tys, subject, keys)?)
                .clone(),
        })
    }
}

pub fn location_layout(
    exec: &mut Exec<'_>, run: &mut Run, emit: (usize, usize), rule: &str, subject: &str,
    keys: &[String], columns: &[crate::pair_summary::LocationColumn], file: u32,
) -> Result<(Arc<Layout>, u32), crate::check_session::refusal::Refusal> {
    let layout = match exec.answer_layouts.entry(emit) {
        Entry::Occupied(slot) => slot.get().clone(),
        Entry::Vacant(slot) => {
            use crate::pair_summary::LocationColumn;
            let (names, types): (Vec<String>, Vec<Ty>) = columns.iter().map(|column| match column {
                LocationColumn::File => ("file".into(), Ty::File),
                LocationColumn::Span => ("span".into(), Ty::Span),
                LocationColumn::Node => ("id".into(), Ty::Node),
            }).unzip();
            slot.insert(Layout::new(run, rule, &names, &types, subject, keys)?).clone()
        }
    };
    let path = run.paths.get(file as usize).ok_or("finding has no source origin")?;
    let origin = layout.words.write().expect("answer words").path(path)?;
    Ok((layout, origin))
}

/// The schema gives each word its type. Node words retain their file and position.
#[derive(Clone)]
pub struct Row {
    pub layout: Arc<Layout>,
    pub words: Box<[u64]>,
    pub place: (u32, u32),
}

pub struct RestoredRows {
    pub dictionary: Arc<RwLock<Words>>,
    pub layouts: Vec<FxHashMap<Vec<Ty>, Arc<Layout>>>,
}
impl RestoredRows {
    pub fn new(words: Vec<Box<str>>, schemas: usize) -> Self {
        let paths: Vec<Arc<str>> = words.into_iter().map(Arc::from).collect();
        let symbols = paths.iter().enumerate().map(|(i, s)| {
            (u32::try_from(i).expect("stored word dictionary"), s.clone())
        }).collect();
        Self {
            dictionary: Arc::new(RwLock::new(Words { paths, symbols, ..Default::default() })),
            layouts: (0..schemas).map(|_| FxHashMap::default()).collect(),
        }
    }

    pub fn row(&mut self, schema: usize, rule: &str, names: &[Box<str>], identity: &[usize], cells: impl Iterator<Item = (Ty, u64)>) -> Row {
        // Null in a numeric column must remain distinct from the number -1.
        let (types, words): (Vec<_>, Vec<_>) = cells.unzip();
        let layout = self.layouts[schema].entry(types).or_insert_with_key(|types| Arc::new(Layout {
            rule: rule.into(),
            names: names.iter().map(|n| n.to_string()).collect(),
            identity: identity.into(),
            types: types.as_slice().into(),
            words: self.dictionary.clone(),
        }));
        Row { layout: layout.clone(), words: words.into(), place: (0, 0) }
    }
}

impl Row {
    pub fn resolved(layout: &Arc<Layout>, words: Box<[u64]>) -> Self {
        Self { layout: layout.clone(), words, place: (0, 0) }
    }

    pub fn decode<T: From<String> + From<&'static str>, D: std::ops::Deref<Target = Words>>(
        &self,
        index: usize,
        dictionary: impl FnOnce() -> D,
        text: impl FnOnce(&Arc<str>) -> T,
    ) -> Option<Cell<T>> {
        let word = self.words[index];
        if word == NULL && self.layout.types[index] != Ty::Num {
            return None;
        }
        Some(match self.layout.types[index] {
            Ty::Sym => Cell::Text(text(&dictionary().symbols[&(word as u32)])),
            Ty::Node => {
                let words = dictionary();
                let path = &words.paths[(word >> 32) as usize];
                Cell::Text(format!("{path}#{}", word as u32).into())
            }
            Ty::File => Cell::File(text(&dictionary().paths[word as usize])),
            Ty::Span => Cell::Span(unpack_span(word)),
            Ty::Num => Cell::Number(Integer(word as i64)),
            Ty::Flag => Cell::Text((if word == 0 { "false" } else { "true" }).into()),
        })
    }

    /// The original byte range of a declared location column. Absence is
    /// distinct from an invalid schema. The typed cell remains in the row.
    pub fn byte_span(&self, column: &str) -> Result<Option<Span>, crate::check_session::refusal::Refusal> {
        let index = self.layout.names.iter().position(|name| name == column)
            .ok_or_else(|| format!("missing location column {column}"))?;
        if self.layout.types[index] != Ty::Span {
            return Err(format!("location column {column} has no span type").into());
        }
        let word = self.words[index];
        if word == NULL {
            return Ok(None);
        }
        let span = unpack_span(word);
        if span == ryo_relations::domain::relations::constants::NO_SPAN {
            return Ok(None);
        }
        if span.start.0 > span.end.0 {
            return Err(format!("location column {column} has a reversed byte range").into());
        }
        Ok(Some(span))
    }

    /// A closed span gets its source from the emitted file subject.
    pub fn span_file(&self) -> Option<Arc<str>> {
        let (&subject, keys) = self.layout.identity.split_first()?;
        if self.layout.types[subject] != Ty::File || !keys.iter().any(|&i| self.layout.types[i] == Ty::Span && self.words[i] != NULL) {
            return None;
        }
        self.layout.words.read().expect("finding dictionary").paths.get(self.words[subject] as usize).cloned()
    }

    /// In the shipping emit contract the first located key (then subject) places
    /// the finding. A position is meaningful only in its own file version.
    pub fn locate(&mut self, path: &str, tree: Option<&crate::languages::NodeTable>, bytes: &[u8], coordinates: &mut crate::languages::contract::FindingCoordinates) -> Result<(), crate::check_session::refusal::Refusal> {
        use crate::languages::{Position, node_table};
        let Some((&subject, keys)) = self.layout.identity.split_first() else { return Err("missing finding subject".into()); };
        if !keys.iter().chain([&subject]).any(|&i| matches!(self.layout.types[i], Ty::Span | Ty::Node)) { return Ok(()); }
        let words = self.layout.words.read().expect("finding dictionary");
        if self.layout.types[subject] == Ty::File && words.paths.get(self.words[subject] as usize).is_none_or(|p| p.as_ref() != path) {
            return Ok(());
        }
        for &column in keys.iter().chain([&subject]) {
            let word = self.words[column];
            if word == NULL { continue; }
            let span = match self.layout.types[column] {
                Ty::Span => self.byte_span(&self.layout.names[column])?,
                Ty::Node => {
                    let Some(tree) = tree else { continue; };
                    let position = Position(word as u32);
                    if words.paths.get((word >> 32) as usize).is_none_or(|p| p.as_ref() != path)
                        || position >= node_table::own_end(tree)
                        || node_table::implied_of(tree, position) == ryo_language::domain::node_table::types::Implied::Yes { continue; }
                    let span = node_table::span_of(tree, position);
                    Some(Span { start: span.start, end: span.end })
                }
                _ => None,
            };
            if let Some(span) = span && span.end.0 as usize <= bytes.len()
                && let Some(point) = coordinates.at(bytes, span.start) {
                self.place = point;
                break;
            }
        }
        Ok(())
    }

    pub fn value(&self, index: usize) -> Value {
        self.decode(index, || self.layout.words.read().expect("answer words"), Arc::clone)
    }

    pub fn cells_with<'a, T: From<String> + From<&'static str>>(
        &'a self,
        text: impl Fn(&Arc<str>) -> T + 'a,
    ) -> impl Iterator<Item = Option<Cell<T>>> + 'a {
        let dictionary = self.layout.words.read().expect("answer words");
        (0..self.words.len()).map(move |i| self.decode(i, || &*dictionary, &text))
    }

    pub fn dictionary(&self) -> &RwLock<Words> {
        &self.layout.words
    }

    pub fn value_from(&self, index: usize, words: &Words) -> Value {
        self.decode(index, || words, Arc::clone)
    }

    pub fn width(&self) -> usize {
        self.words.len()
    }

    pub fn cells(&self) -> impl Iterator<Item = Value> + '_ {
        self.cells_with(Arc::clone)
    }
}

pub fn unpack_span(word: u64) -> Span {
    Span {
        start: Offset((word >> 32) as u32),
        end: Offset(word as u32),
    }
}

impl Words {
    pub fn symbol(&mut self, run: &Run, id: u32) -> Result<&Arc<str>, crate::check_session::refusal::Refusal> {
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
    pub fn path(&mut self, text: &str) -> Result<u32, crate::check_session::refusal::Refusal> {
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

pub fn typed(
    run: &Run,
    table: &Table,
    row: usize,
    file: Option<usize>,
    layout: &Arc<Layout>,
) -> Result<Row, crate::check_session::refusal::Refusal> {
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
        place: (0, 0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_span_after_an_unlocated_node_places_the_finding() {
        let mut restored = RestoredRows::new(vec!["source".into()], 1);
        let mut row = restored.row(0, "rule", &["file".into(), "id".into(), "span".into()],
            &[0, 1, 2], [(Ty::File, 0), (Ty::Node, 1), (Ty::Span, (11 << 32) | 17)].into_iter());
        assert_eq!(row.span_file().as_deref(), Some("source"));
        row.locate("source", None, b"0123456789abcdefg", &mut Default::default()).unwrap();
        assert_eq!(row.place, (1, 12));
        assert_eq!(row.byte_span("span").unwrap(), Some(Span { start: Offset(11), end: Offset(17) }));
    }
}
