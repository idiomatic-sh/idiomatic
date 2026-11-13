//! A file's session over the fused program.

use super::types::Program;
use crate::domain::batch::service::{
    antijoin, cell, closure, each, empty, extend, group_count, join, membership, ordered_match,
    project, push, union, Ordering,
};
use crate::domain::batch::types::Rows;
use crate::domain::fused::types::NodeId;
use crate::outbound::facts::interface::Facts;
use crate::outbound::facts::types::{ElementRow, Kinds};
use ryo_execution_domain::domain::comparing::service::holds_bound;
use ryo_execution_domain::domain::comparing::types::{DecisionWorkspace, RowShape};
use ryo_execution_domain::domain::matching::types::PatternMatcher;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::{Memo, NameCount};
use ryo_execution_domain::domain::table_model::types::Verdict;
use ryo_execution_domain::domain::table_scratch::types::{LazyScratch, SelectScratch};
use ryo_execution_domain::domain::table_texts::service::datum;
use ryo_execution_domain::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_operators::domain::capture::types::{CaptureOutput, CaptureSlots, CaptureSpans, SlotPattern, SlotSubject};
use ryo_operators::domain::contract::errors::ContractError;
use ryo_operators::domain::replace::types::{ReplacedText, ReplacementText};
use ryo_operators::domain::source_matches::types::{
    BaseLine, CaptureRequest, LineBytes, SourceBase, SourceMatchesInput, TextRowDriver,
};
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase as _;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::service::{all, from_bits};
use ryo_relations::domain::relations::types::{
    Bits, CellKind, ColumnsView, Integer, Relation, Value,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    Input, Operand, PlanComparison, QueryLiteral,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::row_texts::types::CellText;

thread_local! {
    /// The select scratch of this thread's sessions: the regular expressions compiled for a comparison stay for the next file.
    static SCRATCH: std::cell::RefCell<LazyScratch<SelectScratch>> =
        std::cell::RefCell::new(LazyScratch(std::cell::OnceCell::new()));
}

/// One file's session.
pub struct Session<'s> {
    program: &'s Program,
    facts: &'s dyn Facts,
    symbols: &'s mut Symbols,
    source: &'s [u8],
    tables: Vec<Option<Result<Rows, OperatingError>>>,
    relations: Vec<Option<Rows>>,
    walked: bool,
}

impl std::fmt::Debug for Session<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(stringify!(Session))
            .finish_non_exhaustive()
    }
}

/// The kind of what a value holds.
fn kind_of(value: Value) -> CellKind {
    match value {
        Value::Node(_) => CellKind::Node,
        Value::Symbol(_) | Value::Name(_) => CellKind::Symbol,
        Value::File(_) => CellKind::File,
        Value::Span(_) => CellKind::Span,
        Value::Number(_) => CellKind::Number,
        Value::Flag(_) => CellKind::Flag,
    }
}

/// Whether every comparison holds for `row`, decided as the interpreter decides it.
fn holds(
    program: &Program,
    comparisons: &[PlanComparison],
    row: &[Value],
    texts: &Texts<'_>,
    scratch: &mut LazyScratch<SelectScratch>,
) -> Result<bool, OperatingError> {
    if comparisons.is_empty() {
        return Ok(true);
    }
    let kinds: Vec<CellKind> = row.iter().copied().map(kind_of).collect();
    let mut memo: Vec<Memo> = Vec::new();
    holds_bound(
        comparisons,
        row,
        RowShape {
            kinds: &kinds,
            names: NameCount(0),
        },
        texts,
        &[],
        None,
        PatternMatcher(&program.matching),
        DecisionWorkspace {
            select: scratch,
            memo: &mut memo,
        },
    )
    .map(|verdict| verdict == Verdict::Holds)
}

/// The rows of a relation the facts port lent, decoded.
fn decoded(view: ColumnsView<'_>) -> Rows {
    let count = view.columns.first().map_or(0, |column| column.0.len());
    let mut rows = empty(view.kinds.len());
    rows.cells.reserve(count * view.kinds.len());
    for at in 0..count {
        for (column, kind) in view.columns.iter().zip(view.kinds) {
            rows.cells
                .push(from_bits(*kind, Bits(column.0.get(at).copied().unwrap_or_default())));
        }
    }
    rows.count = count;
    rows
}

/// Collects the slot ranges of every complete capture.
struct Slots(Vec<Vec<std::ops::Range<usize>>>);
impl CaptureOutput for Slots {
    fn captures(&mut self, ranges: CaptureSpans<'_>) {
        self.0.push(ranges.0.to_vec());
    }
}

/// Collects the occurrences of a source search, interning their captured text in the file's names.
struct Occurrences<'a> {
    rows: &'a mut Rows,
    symbols: &'a mut Symbols,
    source: &'a [u8],
}
impl TextRowDriver for Occurrences<'_> {
    fn intern(&mut self, text: CellText<'_>) -> Value {
        Value::Symbol(intern_derived(
            self.symbols,
            ByteSlice(self.source),
            NameText(text.0),
        ))
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        push(self.rows, row);
        Ok(())
    }
}

impl<'s> Session<'s> {
    /// A session over `facts` for the file whose bytes are `source` and whose names are `symbols`.
    pub fn new(
        program: &'s Program,
        facts: &'s dyn Facts,
        symbols: &'s mut Symbols,
        source: &'s [u8],
    ) -> Self {
        Self {
            program,
            facts,
            symbols,
            source,
            tables: vec![None; program.fused.nodes.len()],
            relations: vec![None; all().len()],
            walked: false,
        }
    }

    /// The one walk over the parser's tree: every root select is served from it at once.
    fn walk(&mut self) -> Result<(), OperatingError> {
        self.walked = true;
        let fused = &self.program.fused;
        let mut tables: Vec<Rows> = fused.roots.iter().map(|_| Rows::default()).collect();
        let mut failure = None;
        let kinds: Vec<_> = fused
            .by_kind
            .iter()
            .enumerate()
            .filter(|(_, roots)| !roots.is_empty())
            .filter_map(|(kind, _)| {
                u16::try_from(kind)
                    .ok()
                    .map(ryo_relations::domain::relations::types::KindBits)
            })
            .collect();
        let walked = if fused.every.is_empty() {
            Kinds::Only(&kinds)
        } else {
            Kinds::Every
        };
        let texts = Texts {
            symbols: self.symbols,
            source: SourceView(self.source),
            paths: &[],
            arenas: &[],
        };
        let program = self.program;
        let facts = self.facts;
        SCRATCH.with_borrow_mut(|scratch| {
            facts.walk(walked, &mut |position, kind| {
                let mut row: Option<ElementRow> = None;
                let fed = fused
                    .by_kind
                    .get(usize::from(kind.0))
                    .into_iter()
                    .flatten()
                    .chain(&fused.every);
                for place in fed {
                    let Some(root) = fused.roots.get(*place) else {
                        continue;
                    };
                    let Some(Operation::Select {
                        comparisons, kept, ..
                    }) = fused.nodes.get(root.node.0)
                    else {
                        continue;
                    };
                    let ElementRow(element) = *row.get_or_insert_with(|| facts.element(position));
                    match holds(program, comparisons, &element, &texts, scratch) {
                        Ok(true) => {
                            if let Some(table) = tables.get_mut(*place) {
                                if kept.is_empty() {
                                    push(table, &element);
                                } else {
                                    let mut made = Vec::with_capacity(kept.len());
                                    if crate::domain::batch::service::pick(&element, kept, &mut made) {
                                        push(table, &made);
                                    }
                                }
                            }
                        }
                        Ok(false) => {}
                        Err(error) => failure = Some(error),
                    }
                }
            });
        });
        if let Some(error) = failure {
            return Err(error);
        }
        for (root, table) in fused.roots.iter().zip(tables) {
            if let Some(slot) = self.tables.get_mut(root.node.0) {
                *slot = Some(Ok(table));
            }
        }
        Ok(())
    }

    /// The rows of `relation`, through the facts port the first time.
    fn relation(&mut self, relation: Relation) -> Result<&Rows, OperatingError> {
        let place = all()
            .iter()
            .position(|each| *each == relation)
            .ok_or(OperatingError::NotRead)?;
        if self.relations.get(place).is_some_and(Option::is_none) {
            let rows = if relation == Relation::SyntaxTree {
                let mut rows = empty(5);
                let facts = self.facts;
                facts.walk(Kinds::Every, &mut |position, _| {
                    push(&mut rows, &facts.element(position).0);
                });
                rows
            } else {
                decoded(
                    self.facts
                        .relation(relation, self.symbols)
                        .map_err(|_| OperatingError::NotRead)?,
                )
            };
            if let Some(slot) = self.relations.get_mut(place) {
                *slot = Some(rows);
            }
        }
        self.relations
            .get(place)
            .and_then(Option::as_ref)
            .ok_or(OperatingError::NotRead)
    }

    /// Makes sure `input`'s rows are there; `true` when it has none.
    fn ready(&mut self, input: Input) -> Result<bool, OperatingError> {
        match input {
            Input::Step(step) => {
                let node = NodeId(usize::from(step));
                self.evaluate(node)?;
                Ok(self.rows_of(node)?.count == 0)
            }
            Input::Relation(relation) => Ok(self.relation(relation)?.count == 0),
            Input::BoundObservation(_) => Err(OperatingError::NotRun),
        }
    }

    /// The rows of an input already made ready.
    fn input(&self, input: Input) -> Result<&Rows, OperatingError> {
        match input {
            Input::Step(step) => self.rows_of(NodeId(usize::from(step))),
            Input::Relation(relation) => all()
                .iter()
                .position(|each| *each == relation)
                .and_then(|place| self.relations.get(place))
                .and_then(Option::as_ref)
                .ok_or(OperatingError::NotRead),
            Input::BoundObservation(_) => Err(OperatingError::NotRun),
        }
    }

    /// The rows of a step already evaluated.
    fn rows_of(&self, node: NodeId) -> Result<&Rows, OperatingError> {
        match self.tables.get(node.0) {
            Some(Some(Ok(rows))) => Ok(rows),
            Some(Some(Err(error))) => Err(error.clone()),
            Some(None) | None => Err(OperatingError::NotRun),
        }
    }

    /// The rows the step `node` answers for the file, made the first time they are asked for.
    ///
    /// # Errors
    ///
    /// What the step's meaning refuses.
    pub fn rows(&mut self, node: NodeId) -> Result<&Rows, OperatingError> {
        self.evaluate(node)?;
        self.rows_of(node)
    }

    /// Evaluates `node` and the steps it reads, once.
    fn evaluate(&mut self, node: NodeId) -> Result<(), OperatingError> {
        if matches!(self.tables.get(node.0), Some(Some(_))) {
            return Ok(());
        }
        let program = self.program;
        let operation = program.fused.nodes.get(node.0).ok_or(OperatingError::NotRun)?;
        if matches!(
            operation,
            Operation::Select {
                from: Input::Relation(Relation::SyntaxTree),
                ..
            }
        ) && !self.walked
        {
            self.walk()?;
            if matches!(self.tables.get(node.0), Some(Some(_))) {
                return Ok(());
            }
        }
        let answer = self.answer(node, operation);
        if let Some(slot) = self.tables.get_mut(node.0) {
            *slot = Some(answer);
        }
        Ok(())
    }

    /// The text of `value`, owned.
    fn text(&self, value: Value) -> Option<String> {
        let texts = Texts {
            symbols: self.symbols,
            source: SourceView(self.source),
            paths: &[],
            arenas: &[],
        };
        match datum(value, &texts) {
            Datum::Text(text) => Some(text.0.to_owned()),
            Datum::Unlocated
            | Datum::Number(_)
            | Datum::Flag(_)
            | Datum::Numbers(_)
            | Datum::Texts(_) => None,
        }
    }

    /// The regular expression compiled for `node`.
    fn regex(&self, node: NodeId) -> Result<&ryo_matching::inbound::use_cases::matching::types::CompiledRegex, OperatingError> {
        match self.program.regexes.get(node.0) {
            Some(Some(Ok(regex))) => Ok(regex),
            Some(Some(Err(_)) | None) | None => Err(OperatingError::NotRun),
        }
    }

    /// The rows `operation`, the step `node`, answers.
    #[expect(clippy::too_many_lines, reason = "one arm for each operation the spike evaluates")]
    fn answer(&mut self, node: NodeId, operation: &Operation) -> Result<Rows, OperatingError> {
        match operation {
            Operation::Select {
                from,
                comparisons,
                kept,
            } => {
                if self.ready(*from)? {
                    return Ok(empty(kept.len()));
                }
                let texts = Texts {
                    symbols: self.symbols,
                    source: SourceView(self.source),
                    paths: &[],
                    arenas: &[],
                };
                let rows = self.input(*from)?;
                let mut answer = empty(if kept.is_empty() { rows.width } else { kept.len() });
                let mut made = Vec::with_capacity(kept.len());
                SCRATCH.with_borrow_mut(|scratch| {
                    for each in each(rows) {
                        if !holds(self.program, comparisons, each, &texts, scratch)? {
                            continue;
                        }
                        if kept.is_empty() {
                            push(&mut answer, each);
                        } else {
                            made.clear();
                            if crate::domain::batch::service::pick(each, kept, &mut made) {
                                push(&mut answer, &made);
                            }
                        }
                    }
                    Ok::<(), OperatingError>(())
                })?;
                Ok(answer)
            }
            Operation::Project {
                from,
                columns,
                distinct,
            } => {
                if self.ready(*from)? {
                    return Ok(empty(columns.len()));
                }
                Ok(project(self.input(*from)?, columns, *distinct))
            }
            Operation::Extend { from, values } => {
                if self.ready(*from)? {
                    return Ok(Rows::default());
                }
                let mut appended = Vec::with_capacity(values.len());
                for value in values {
                    appended.push(match value {
                        Operand::Literal(QueryLiteral::Number(number)) => Value::Number(*number),
                        Operand::Literal(QueryLiteral::Text(text)) => Value::Symbol(intern_derived(
                            self.symbols,
                            ByteSlice(self.source),
                            NameText(&text.0),
                        )),
                        Operand::Column(_) | Operand::Parameter(_) | Operand::Function(_) => {
                            return Err(OperatingError::NotRun);
                        }
                    });
                }
                Ok(extend(self.input(*from)?, &appended))
            }
            Operation::Join {
                left,
                right,
                left_on,
                right_on,
                output,
                filter,
                view,
                reach: _,
            } => {
                if self.ready(*left)? || self.ready(*right)? {
                    return Ok(empty(output.len()));
                }
                let right_rows = if filter.is_empty() {
                    None
                } else {
                    let texts = Texts {
                        symbols: self.symbols,
                        source: SourceView(self.source),
                        paths: &[],
                        arenas: &[],
                    };
                    let rows = self.input(*right)?;
                    let mut kept = empty(rows.width);
                    SCRATCH.with_borrow_mut(|scratch| {
                        for each in each(rows) {
                            if holds(self.program, filter, each, &texts, scratch)? {
                                push(&mut kept, each);
                            }
                        }
                        Ok::<(), OperatingError>(())
                    })?;
                    Some(kept)
                };
                let right_rows = match &right_rows {
                    Some(rows) => rows,
                    None => self.input(*right)?,
                };
                Ok(join(
                    self.input(*left)?,
                    right_rows,
                    left_on,
                    right_on,
                    output,
                    view,
                ))
            }
            Operation::Union { left, right } => {
                let _: bool = self.ready(*left)?;
                let _: bool = self.ready(*right)?;
                Ok(union(self.input(*left)?, self.input(*right)?))
            }
            Operation::Intersection { left, right } | Operation::Difference { left, right } => {
                if self.ready(*left)? {
                    return Ok(Rows::default());
                }
                let _: bool = self.ready(*right)?;
                Ok(membership(
                    self.input(*left)?,
                    self.input(*right)?,
                    matches!(operation, Operation::Intersection { .. }),
                ))
            }
            Operation::Antijoin {
                left,
                right,
                left_on,
                right_on,
            } => {
                if self.ready(*left)? {
                    return Ok(Rows::default());
                }
                let _: bool = self.ready(*right)?;
                Ok(antijoin(
                    self.input(*left)?,
                    self.input(*right)?,
                    left_on,
                    right_on,
                ))
            }
            Operation::GroupCount {
                from,
                keys,
                distinct,
            } => {
                if self.ready(*from)? {
                    return Ok(empty(keys.len() + 1));
                }
                Ok(group_count(self.input(*from)?, keys, *distinct))
            }
            Operation::Closure {
                edges,
                keys,
                source,
                destination,
            } => {
                if self.ready(*edges)? {
                    return Ok(empty(keys.len() + 2));
                }
                Ok(closure(self.input(*edges)?, keys, *source, *destination))
            }
            Operation::OrderedMatch {
                left,
                right,
                left_on,
                right_on,
                left_order,
                right_order,
                output,
            } => {
                if self.ready(*left)? || self.ready(*right)? {
                    return Ok(empty(output.len()));
                }
                Ok(ordered_match(
                    self.input(*left)?,
                    self.input(*right)?,
                    Ordering {
                        left_on,
                        right_on,
                        left_order: *left_order,
                        right_order: *right_order,
                        output,
                    },
                ))
            }
            Operation::SpanBounds { from, span } => {
                if self.ready(*from)? {
                    return Ok(Rows::default());
                }
                let rows = self.input(*from)?;
                let mut answer = empty(rows.width + 2);
                let mut made = Vec::with_capacity(answer.width);
                for each in each(rows) {
                    let Some(Value::Span(located)) = cell(each, *span) else {
                        return Err(OperatingError::ShapeMismatch);
                    };
                    if located == NO_SPAN || located.start > located.end {
                        return Err(OperatingError::NotRun);
                    }
                    made.clear();
                    made.extend_from_slice(each);
                    made.push(Value::Number(Integer(i64::from(located.start.0))));
                    made.push(Value::Number(Integer(i64::from(located.end.0))));
                    push(&mut answer, &made);
                }
                Ok(answer)
            }
            Operation::Capture {
                from,
                path,
                pattern,
            } => {
                if self.ready(*from)? {
                    return Ok(Rows::default());
                }
                let rows = self.input(*from)?.clone();
                let width = pattern.0.bytes().filter(|byte| *byte == b'{').count();
                let mut answer = empty(rows.width + width);
                let mut made = Vec::new();
                for each in each(&rows) {
                    let Some(text) = cell(each, *path).and_then(|value| self.text(value)) else {
                        continue;
                    };
                    let mut found = Slots(Vec::new());
                    let mut slots = Vec::new();
                    let _answer = self
                        .program
                        .operators
                        .operators(OperatorCall::Capture {
                            pattern: SlotPattern(&pattern.0),
                            text: SlotSubject(&text),
                            slots: CaptureSlots(&mut slots),
                            each: &mut found,
                        })
                        .map_err(|_| OperatingError::NotRun)?;
                    for way in found.0 {
                        made.clear();
                        made.extend_from_slice(each);
                        for range in way {
                            made.push(Value::Symbol(intern_derived(
                                self.symbols,
                                ByteSlice(self.source),
                                NameText(text.get(range).unwrap_or_default()),
                            )));
                        }
                        push(&mut answer, &made);
                    }
                }
                Ok(answer)
            }
            Operation::Replace {
                from,
                text,
                pattern: _,
                replacement,
            } => {
                if self.ready(*from)? {
                    return Ok(Rows::default());
                }
                let rows = self.input(*from)?.clone();
                let mut answer = empty(rows.width + 1);
                let mut made = Vec::new();
                let mut replaced = String::new();
                for each in each(&rows) {
                    let Some(subject) = cell(each, *text).and_then(|value| self.text(value)) else {
                        return Err(OperatingError::ShapeMismatch);
                    };
                    let regex = self.regex(node)?;
                    let _answer = self
                        .program
                        .operators
                        .operators(OperatorCall::Replace {
                            regex,
                            text: CellText(&subject),
                            replacement: ReplacementText(&replacement.0),
                            target: ReplacedText(&mut replaced),
                        })
                        .map_err(|_| OperatingError::NotRun)?;
                    made.clear();
                    made.extend_from_slice(each);
                    made.push(Value::Symbol(intern_derived(
                        self.symbols,
                        ByteSlice(self.source),
                        NameText(&replaced),
                    )));
                    push(&mut answer, &made);
                }
                Ok(answer)
            }
            Operation::SourceMatches {
                from,
                span,
                base_line,
                pattern: _,
                capture,
                capture_text,
                capture_line,
            } => {
                if self.ready(*from)? {
                    return Ok(Rows::default());
                }
                let rows = self.input(*from)?.clone();
                let program = self.program;
                let regex = match program.regexes.get(node.0) {
                    Some(Some(Ok(regex))) => regex,
                    Some(Some(Err(_)) | None) | None => return Err(OperatingError::NotRun),
                };
                let source = self.source;
                let mut answer = Rows::default();
                for each in each(&rows) {
                    let Some(Value::Span(located)) = cell(each, *span) else {
                        return Err(OperatingError::ShapeMismatch);
                    };
                    if located == NO_SPAN {
                        continue;
                    }
                    let start = usize::try_from(located.start.0)
                        .map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
                    let end = usize::try_from(located.end.0)
                        .map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
                    if start > end {
                        return Err(OperatingError::InvalidSourceMatchSpan);
                    }
                    let checked = std::str::from_utf8(
                        source
                            .get(start..end)
                            .ok_or(OperatingError::InvalidSourceMatchSpan)?,
                    )
                    .map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
                    let line = match base_line {
                        Some(column) => {
                            let Some(Value::Number(number)) = cell(each, *column) else {
                                return Err(OperatingError::ShapeMismatch);
                            };
                            if number.0 <= 0 {
                                return Err(OperatingError::InvalidSourceMatchSpan);
                            }
                            BaseLine::Starts(number)
                        }
                        None => BaseLine::Unnumbered,
                    };
                    let mut target = Vec::new();
                    let mut driver = Occurrences {
                        rows: &mut answer,
                        symbols: self.symbols,
                        source,
                    };
                    let _answer = program
                        .operators
                        .operators(OperatorCall::SourceMatches {
                            input: SourceMatchesInput {
                                row: each,
                                source: LineBytes(source),
                                base: SourceBase(start),
                                text: CellText(checked),
                                line,
                                regex,
                                capture: capture
                                    .as_ref()
                                    .map_or(CaptureRequest::Whole, |name| {
                                        CaptureRequest::Group(CellText(&name.0))
                                    }),
                                capture_text: *capture_text,
                                capture_line: *capture_line,
                            },
                            driver: &mut driver,
                            target: &mut target,
                        })
                        .map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
                }
                Ok(answer)
            }
            Operation::GroupDigest { .. }
            | Operation::Rank { .. }
            | Operation::Words { .. }
            | Operation::Axis { .. }
            | Operation::OrderedDfs { .. }
            | Operation::Cycles { .. }
            | Operation::Rows { .. }
            | Operation::Hosted { .. }
            | Operation::SpanBetween { .. }
            | Operation::RenderSyntax { .. }
            | Operation::FitsRendered { .. } => Err(OperatingError::NotRun),
        }
    }
}

