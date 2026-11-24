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
#[path = "named_deferred_resolution.rs"]
mod deferred_resolution;
#[path = "named_deferred_visit.rs"]
mod deferred_visit;
#[path = "named_type_expression.rs"]
mod type_expression;

struct Vocabulary {
    words: l::FileWords,
    kind_sym: Vec<u32>,
    field_sym: Vec<u32>,
    empty: u32,
}

impl Vocabulary {
    fn new() -> Self {
        let mut words =
            l::facts::file_words(&l::facts::base_words(l::CheckedNames::default(), &[]));
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

    fn text(&self, word: u64) -> &[u8] {
        l::facts::text(&self.words, l::CellWord(word)).0.as_bytes()
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

const TYPE: FileRelation = FileRelation {
    name: "type-expression",
    columns: &[
        ("id", CellKind::Node),
        ("kind", CellKind::Symbol),
        ("file", CellKind::File),
    ],
};
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
const VISIT: FileRelation = FileRelation {
    name: "deferred-visit",
    columns: &[
        ("id", CellKind::Node),
        ("visit", CellKind::Symbol),
        ("file", CellKind::File),
    ],
};
const RESOLUTION: FileRelation = FileRelation {
    name: "deferred-resolution",
    columns: &[
        ("reference", CellKind::Node),
        ("occurrence", CellKind::Node),
        ("kind", CellKind::Symbol),
        ("point", CellKind::Number),
        ("file", CellKind::File),
    ],
};

const PREDECESSOR: FileRelation = FileRelation {
    name: "scheduled-read-predecessor",
    columns: &[
        ("reference", CellKind::Node),
        ("event", CellKind::Node),
        ("kind", CellKind::Symbol),
        ("namespace", CellKind::Symbol),
        ("scope", CellKind::Node),
        ("depth", CellKind::Number),
        ("ordinal", CellKind::Number),
        ("context", CellKind::Symbol),
        ("file", CellKind::File),
    ],
};

impl ColumnFile<'_, '_> {
    /// Complete file-local typing and postponed-evaluation relations.
    pub const TYPING_SCHEMAS: &'static [FileRelation] =
        &[TYPE, ANNOTATION, VISIT, RESOLUTION, PREDECESSOR];

    /// Answer the requested local semantic projections and reuse dependencies within the request.
    /// Typing inputs carry the explicit configured generic heads and argument forms.
    pub fn visit_typing_relations(
        &mut self,
        schemas: &[FileRelation],
        file: l::FileId,
        stub: bool,
        generic: &dyn Fn(&[u8]) -> bool,
        argument_forms: &dyn Fn(&[u8]) -> bool,
        modules: &[Vec<u8>],
        mut output: impl FnMut(&FileRelation, Result<Vec<Vec<u64>>, String>, &l::FileWords),
    ) {
        let mut semantic = Semantics {
            file: self,
            number: u64::from(file.0),
            stub,
            lists: type_expression::TypeExpressionInputs {
                generic,
                argument_forms,
                modules,
            },
            words: Vocabulary::new(),
            inputs: HashMap::new(),
        };
        for schema in schemas {
            let rows = if !Self::TYPING_SCHEMAS.contains(schema) {
                Err(format!("no semantic relation {}", schema.name))
            } else if semantic.file.file.syntax.completeness.syntax
                != l::SyntaxCompleteness::Complete
            {
                Err(format!("incomplete {}", schema.name))
            } else {
                semantic.answer(*schema)
            };
            output(schema, rows.map(|table| table.cols), &semantic.words.words);
        }
    }
}

struct Semantics<'a, 'p, 'f> {
    file: &'a mut ColumnFile<'p, 'f>,
    number: u64,
    stub: bool,
    lists: type_expression::TypeExpressionInputs<'a>,
    words: Vocabulary,
    inputs: HashMap<&'static str, InputTable>,
}

impl Semantics<'_, '_, '_> {
    fn answer(&mut self, schema: FileRelation) -> Result<InputTable, String> {
        match schema.name {
            "type-expression" => self.types(),
            "annotation-evaluation" => self.annotations(),
            "deferred-visit" => self.visits(),
            "deferred-resolution" => self.resolution(),
            "scheduled-read-predecessor" => self.predecessor(),
            _ => unreachable!("semantic schema was checked"),
        }
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
            self.inputs.insert(name, table);
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

    fn types(&mut self) -> Result<InputTable, String> {
        use l::{
            CallArgumentColumn::{Call, Expression, Position},
            IdentifierColumn::{Id, Name},
            QualifiedNameColumn,
        };
        let mut output = InputTable::empty(&TYPE);
        self.embedded()?;
        self.read::<l::QualifiedName>(
            "qualified",
            &[
                ("id", QualifiedNameColumn::Id),
                ("path", QualifiedNameColumn::Path),
            ],
        )?;
        if !type_expression::any_root(
            &mut self.words,
            self.file.table,
            &self.inputs["qualified"],
            self.inputs["embedded"].n,
            &self.lists,
        )? {
            return Ok(output);
        }
        self.read::<l::CallArgument>(
            "arguments",
            &[
                ("call", Call),
                ("expression", Expression),
                ("position", Position),
            ],
        )?;
        self.read::<l::Identifier>("identifiers", &[("id", Id), ("name", Name)])?;
        let inputs = type_expression::Inputs {
            qualified: &self.inputs["qualified"],
            arguments: &self.inputs["arguments"],
            identifiers: &self.inputs["identifiers"],
            embedded: &self.inputs["embedded"],
            lists: &self.lists,
        };
        let rows = type_expression::rows(&mut self.words, self.file.table, &inputs)?;
        let ty = u64::from(self.words.symbol(b"type"));
        let value = u64::from(self.words.symbol(b"value"));
        for (id, typed) in rows {
            output.push(&[id.into(), if typed { ty } else { value }, self.number]);
        }
        Ok(output)
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

    fn visits(&mut self) -> Result<InputTable, String> {
        let mut output = InputTable::empty(&VISIT);
        let evaluations = self.annotations()?;
        self.embedded()?;
        let future = self.features()?.enabled;
        if !self.stub
            && self.inputs["embedded"].n == 0
            && !deferred_visit::lazy_any(&mut self.words, &evaluations, future)?
            && !deferred_visit::has_type_parameters(&mut self.words, self.file.table)
        {
            return Ok(output);
        }
        let types = if self.stub {
            self.types()?
        } else {
            InputTable::empty(&TYPE)
        };
        let inputs = deferred_visit::Inputs {
            types: &types,
            evaluations: &evaluations,
            embedded: &self.inputs["embedded"],
            stub: self.stub,
            future,
        };
        let rows = deferred_visit::rows(&mut self.words, self.file.table, &inputs)?;
        let deferred = u64::from(self.words.symbol(b"deferred"));
        for id in rows {
            output.push(&[id.into(), deferred, self.number]);
        }
        Ok(output)
    }

    fn resolution(&mut self) -> Result<InputTable, String> {
        use l::BindingEventColumn::{Kind, Occurrence, Ordinal};
        use l::BindingReadColumn::{Name, Reference, Scope};
        use l::ImplicitSymbolColumn;
        let mut output = InputTable::empty(&RESOLUTION);
        let visits = self.visits()?;
        if visits.n == 0 {
            return Ok(output);
        }
        self.read::<l::BindingRead>(
            "reads",
            &[("reference", Reference), ("name", Name), ("scope", Scope)],
        )?;
        self.read::<l::BindingEvent>(
            "events",
            &[
                ("occurrence", Occurrence),
                ("ordinal", Ordinal),
                ("kind", Kind),
            ],
        )?;
        self.read::<l::ImplicitSymbol>(
            "implicit",
            &[
                ("name", ImplicitSymbolColumn::Name),
                ("kind", ImplicitSymbolColumn::Kind),
                ("since-major", ImplicitSymbolColumn::SinceMajor),
                ("since-minor", ImplicitSymbolColumn::SinceMinor),
            ],
        )?;
        self.targets()?;
        let schema = ColumnFile::SITE_SCHEMAS
            .iter()
            .find(|schema| schema.name == "namespace-final")
            .expect("site schema");
        let mut finals = InputTable::empty(schema);
        self.file.visit_site_relation(
            schema.name,
            self.number,
            |text| u64::from(self.words.symbol(text.as_bytes())),
            |row| finals.push(row),
        )?;
        let inputs = deferred_resolution::Inputs {
            reads: &self.inputs["reads"],
            events: &self.inputs["events"],
            implicit: &self.inputs["implicit"],
            targets: &self.inputs["targets"],
            finals: &finals,
            visits: &visits,
        };
        let rows = deferred_resolution::rows(&mut self.words, self.file.table, &inputs)?;
        for (reference, occurrence, kind, point) in rows {
            output.push(&[reference, occurrence, kind, point, self.number]);
        }
        Ok(output)
    }

    fn predecessor(&mut self) -> Result<InputTable, String> {
        use l::{BindingReadColumn as Read, BindingResolutionColumn as Lookup};
        self.read::<l::BindingResolution>(
            "lookups",
            &[
                ("reference", Lookup::Reference),
                ("event", Lookup::Event),
                ("kind", Lookup::Kind),
                ("scope", Lookup::Scope),
                ("lexical", Lookup::Lexical),
                ("depth", Lookup::Depth),
                ("ordinal", Lookup::Ordinal),
                ("context", Lookup::Context),
            ],
        )?;
        let read = InputTable::read::<l::BindingRead>(
            self.file,
            &mut self.words.words,
            &[
                ("reference", Read::Reference),
                ("scope", Read::Scope),
                ("context", Read::Context),
            ],
        )?;
        let [deletion, own, routed, none] = [b"deletion".as_slice(), b"own", b"routed", b"none"]
            .map(|text| u64::from(self.words.symbol(text)));
        let lookup = &self.inputs["lookups"];
        let mut out = InputTable::empty(&PREDECESSOR);
        ColumnFile::visit_scheduled_predecessors(
            (0..lookup.n).map(|i| std::array::from_fn(|column| lookup.cols[column][i])),
            (0..read.n).map(|i| std::array::from_fn(|column| read.cols[column][i])),
            [deletion, own, routed, none],
            self.number,
            |row| out.push(&row),
        );
        Ok(out)
    }
}

impl ColumnFile<'_, '_> {
    /// Report the first eligible resolver row per read, including reads without any event.
    pub fn visit_scheduled_predecessors(
        lookups: impl Iterator<Item = [u64; 8]>,
        reads: impl ExactSizeIterator<Item = [u64; 3]>,
        [deletion, own, routed, none]: [u64; 4],
        file: u64,
        mut emit: impl FnMut([u64; 9]),
    ) {
        const NULL: u64 = u64::MAX;
        let mut seen = std::collections::HashSet::with_capacity(reads.len());
        for [
            reference,
            event,
            kind,
            scope,
            lexical,
            depth,
            ordinal,
            context,
        ] in lookups
        {
            if (context == deletion && depth != 0) || !seen.insert(reference) {
                continue;
            }
            let namespace = if lexical == scope { own } else { routed };
            emit([
                reference, event, kind, namespace, scope, depth, ordinal, context, file,
            ]);
        }
        for [reference, scope, context] in reads {
            if seen.insert(reference) {
                emit([
                    reference, NULL, none, none, scope, NULL, NULL, context, file,
                ]);
            }
        }
    }
}
