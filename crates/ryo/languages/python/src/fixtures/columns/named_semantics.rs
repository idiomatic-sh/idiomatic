//! File-scoped semantic projections of the numbered Python walk.
use super::ColumnFile;
use crate::domain::{
    labels::service::{field_name, fields},
    syntax::service::{kind_name, kinds},
};
use ryo::languages::{self as l, CatalogRelation, CellKind, FileRelation};
use std::collections::HashMap;

#[path = "named_annotation_evaluation.rs"]
mod annotation_evaluation;

struct Vocabulary {
    words: l::FileWords,
    kind_sym: Vec<u32>,
    field_sym: Vec<u32>,
    empty: u32,
}

impl Vocabulary {
    fn new(mut words: l::FileWords) -> Self {
        let empty = l::facts::intern(&mut words, l::NameText("")).0 as u32;
        let kind_sym = kinds()
            .map(|kind| l::facts::intern(&mut words, kind_name(kind)).0 as u32)
            .collect();
        let field_sym = fields()
            .map(|field| l::facts::intern(&mut words, field_name(field)).0 as u32)
            .collect();
        Self {
            words,
            kind_sym,
            field_sym,
            empty,
        }
    }

    fn symbol(&mut self, text: &[u8]) -> u32 {
        let text = std::str::from_utf8(text).expect("Python syntax vocabulary");
        l::facts::intern(&mut self.words, l::NameText(text)).0 as u32
    }
}

struct InputTable {
    names: Vec<&'static str>,
    tys: Vec<CellKind>,
    cols: Vec<Vec<u64>>,
    n: usize,
}

impl InputTable {
    fn empty(schema: &FileRelation) -> Self {
        let (names, tys) = schema.columns.iter().copied().unzip();
        Self {
            names,
            tys,
            cols: vec![Vec::new(); schema.columns.len()],
            n: 0,
        }
    }

    fn push(&mut self, row: &[u64]) {
        for (column, &cell) in self.cols.iter_mut().zip(row) {
            column.push(cell);
        }
        self.n += 1;
    }

    fn read<R: CatalogRelation>(
        file: &mut ColumnFile<'_, '_>,
        words: &mut l::FileWords,
        projection: &[(&'static str, R::Column)],
    ) -> Result<Self, String>
    where
        R::Column: PartialEq,
    {
        let mut filled = l::FilledColumns::default();
        let relation = R::relation();
        if !matches!(
            file.fill(relation, l::Wanted::Whole, words, &mut filled),
            l::FactAnswer::Answered(_)
        ) {
            return Err(format!("incomplete {relation:?}"));
        }
        let columns = R::columns();
        let n = filled.0.first().map_or(0, |column| column.0.len());
        let mut result = Self {
            names: Vec::new(),
            tys: Vec::new(),
            cols: Vec::new(),
            n,
        };
        for &(name, column) in projection {
            let index = columns
                .iter()
                .position(|each| *each == column)
                .expect("catalog column");
            let kind = R::kind(column);
            let values = filled.0.get(index).map_or(&[][..], |column| &column.0);
            if values.len() != n {
                return Err(format!("incomplete {relation:?}.{name}"));
            }
            let cells = values.iter().map(|cell| match kind {
                CellKind::Node if cell.0 != u64::MAX => u64::from(cell.0 as u32),
                _ => cell.0,
            });
            result.names.push(name);
            result.tys.push(kind);
            result.cols.push(cells.collect());
        }
        Ok(result)
    }
}

fn col(table: &InputTable, name: &str) -> Result<usize, String> {
    table
        .names
        .iter()
        .position(|column| *column == name)
        .ok_or_else(|| format!("no column {name}"))
}

const ANNOTATION: FileRelation = FileRelation {
    name: "annotation-evaluation",
    columns: &[
        ("id", CellKind::Node),
        ("root", CellKind::Node),
        ("evaluation", CellKind::Symbol),
        ("mode", CellKind::Symbol),
        ("file", CellKind::File),
    ],
};
impl ColumnFile<'_, '_> {
    /// Complete file-local typing relations.
    pub const TYPING_SCHEMAS: &'static [FileRelation] = &[ANNOTATION];

    /// Answer the requested local semantic projections and reuse dependencies within the request.
    pub fn visit_typing_relations(
        &mut self,
        schemas: &[FileRelation],
        file: l::FileId,
        mut output: impl FnMut(&FileRelation, Result<Vec<Vec<u64>>, String>, &l::FileWords),
    ) {
        // Walk fillers encode fixed words by their declared vocabulary index.
        // Their answers and the semantic projection must use the walk's dictionary.
        let words = Vocabulary::new(std::mem::take(self.words));
        let mut semantic = Semantics {
            file: self,
            number: u64::from(file.0),
            words,
            inputs: HashMap::new(),
        };
        // Compute dependencies once, then move each requested table into its answer.
        let prepared: Vec<_> = schemas
            .iter()
            .map(|schema| {
                if !Self::TYPING_SCHEMAS.contains(schema) {
                    Err(format!("no semantic relation {}", schema.name))
                } else if semantic.file.file.syntax.completeness.syntax
                    != l::SyntaxCompleteness::Complete
                {
                    Err(format!("incomplete {}", schema.name))
                } else {
                    semantic.prepare(*schema)
                }
            })
            .collect();
        for (index, (schema, prepared)) in schemas.iter().zip(prepared).enumerate() {
            let rows = prepared.map(|()| {
                if schemas[index + 1..].contains(schema) {
                    semantic.inputs[schema.name].cols.clone()
                } else {
                    semantic
                        .inputs
                        .remove(schema.name)
                        .expect("prepared semantic relation")
                        .cols
                }
            });
            output(schema, rows, &semantic.words.words);
        }
        *semantic.file.words = semantic.words.words;
    }
}

struct Semantics<'a, 'p, 'f> {
    file: &'a mut ColumnFile<'p, 'f>,
    number: u64,
    words: Vocabulary,
    inputs: HashMap<&'static str, InputTable>,
}

impl Semantics<'_, '_, '_> {
    fn prepare(&mut self, schema: FileRelation) -> Result<(), String> {
        if self.inputs.contains_key(schema.name) {
            return Ok(());
        }
        let rows = match schema.name {
            "annotation-evaluation" => self.annotations(),
            _ => unreachable!("semantic schema was checked"),
        }?;
        let _ = self.inputs.insert(schema.name, rows);
        Ok(())
    }

    fn read<R: CatalogRelation>(
        &mut self,
        name: &'static str,
        projection: &[(&'static str, R::Column)],
    ) -> Result<(), String>
    where
        R::Column: PartialEq,
    {
        if !self.inputs.contains_key(name) {
            let table = InputTable::read::<R>(self.file, &mut self.words.words, projection)?;
            let _ = self.inputs.insert(name, table);
        }
        Ok(())
    }

    fn targets(&mut self) -> Result<(), String> {
        use l::LanguageTargetColumn::{Major, Minor};
        self.read::<l::LanguageTarget>("targets", &[("major", Major), ("minor", Minor)])
    }

    fn embedded(&mut self) -> Result<(), String> {
        use l::EmbeddedExpressionColumn::Root;
        self.read::<l::EmbeddedExpression>("embedded", &[("root", Root)])
    }

    fn features(&mut self) -> Result<annotation_evaluation::Target, String> {
        use l::CompilerFeatureColumn::{Enabled, Feature};
        self.targets()?;
        self.read::<l::CompilerFeature>("features", &[("feature", Feature), ("enabled", Enabled)])?;
        annotation_evaluation::target(
            &mut self.words,
            &self.inputs["features"],
            &self.inputs["targets"],
        )
    }

    fn annotations(&mut self) -> Result<InputTable, String> {
        let mut output = InputTable::empty(&ANNOTATION);
        if !annotation_evaluation::has_annotation(&mut self.words, self.file.table) {
            return Ok(output);
        }
        let target = self.features()?;
        self.embedded()?;
        let rows = annotation_evaluation::rows(
            &mut self.words,
            self.file.table,
            &target,
            &self.inputs["embedded"],
        )?;
        for (id, root, evaluation, mode) in rows {
            let evaluation = u64::from(self.words.symbol(evaluation));
            let mode = u64::from(self.words.symbol(mode));
            output.push(&[id.into(), root.into(), evaluation, mode, self.number]);
        }
        Ok(output)
    }
}
