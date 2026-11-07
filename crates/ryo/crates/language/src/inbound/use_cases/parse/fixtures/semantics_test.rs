//! Real conformance refusal paths for a semantic session's declared outcome and reason.

use super::judge;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::builders::service::{enter_node, leave_node, table_builder};
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::semantics::errors::SemanticsError;
use crate::domain::semantics::types::SemanticSession;
use crate::domain::semantics::types::{
    RenderContext, RenderOutcome, RenderRequest, RenderedNode, SemanticSlot,
};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use crate::inbound::use_cases::parse::errors::ParseError;
use crate::inbound::use_cases::parse::fixtures::expected::SemanticExpected;
use crate::inbound::use_cases::parse::fixtures::findings::Findings;
use crate::inbound::use_cases::parse::types::{
    KindNames, ParseConfigurationIdentity, ParseEngine, ParseRequest, Recognition,
    SkippedDirectory, SourceFile,
};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{
    Count, FileId, KindBits, Offset, Relation, RowWriter,
};
use ryo_relations::domain::terms::types::{ReasonName, UnknownName, UnknownReason};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Scenario {
    WrongOutcome,
    NoSession,
    UndeclaredReason,
}

struct FixtureParse(Scenario);
struct FixtureSession(Scenario);

const REASONS: &[UnknownName] = &[UnknownName {
    reason: UnknownReason(1),
    text: ReasonName("render-unsupported"),
}];

impl SemanticSession for FixtureSession {
    fn render_batch(
        &mut self,
        _source: ByteSlice<'_>,
        requests: &[RenderRequest],
    ) -> Result<Vec<RenderOutcome>, SemanticsError> {
        let reason = match self.0 {
            Scenario::UndeclaredReason => UnknownReason(2),
            Scenario::WrongOutcome | Scenario::NoSession => UnknownReason(1),
        };
        Ok(vec![RenderOutcome::Unsupported(reason); requests.len()])
    }
}

impl ParseEngine for FixtureParse {
    fn capabilities(
        &self,
    ) -> &'static [ryo_relations::domain::relations::types::OptionalCapability] {
        &[]
    }
    fn configuration_identity(&self) -> ParseConfigurationIdentity<'_> {
        ParseConfigurationIdentity("")
    }
    fn begin_semantics(
        &self,
        _request: &ParseRequest<'_>,
        _tree: &Elements,
        slot: &mut SemanticSlot,
    ) {
        match self.0 {
            Scenario::NoSession => {}
            Scenario::WrongOutcome | Scenario::UndeclaredReason => {
                *slot = SemanticSlot::Begun(Box::new(FixtureSession(self.0)));
            }
        }
    }
    fn recognition(&self, _request: &ParseRequest<'_>) -> Recognition {
        Recognition::Reads
    }
    fn skips(&self) -> &'static [SkippedDirectory] {
        &[]
    }
    fn kind_count(&self) -> Count {
        Count(2)
    }
    fn kind_names(&self) -> KindNames {
        KindNames(Vec::new())
    }
    fn unknown_reasons(&self) -> &'static [UnknownName] {
        REASONS
    }
    fn reader_kinds(&self) -> Vec<KindBits> {
        Vec::new()
    }
    fn build_file(
        &self,
        request: ParseRequest<'_>,
        table: &mut Elements,
        symbols: &mut Symbols,
    ) -> Result<(), ParseError> {
        let mut builder = table_builder(table, symbols, request.bytes);
        enter_node(&mut builder, KindBits(1), Offset(0));
        leave_node(&mut builder, Offset(4));
        Ok(())
    }
    fn baseline_file(&self, _request: &ParseRequest<'_>) -> Result<(), ParseError> {
        Ok(())
    }
    fn read_file(
        &self,
        _request: ParseRequest<'_>,
        _table: &mut Elements,
        _symbols: &mut Symbols,
        _wanted: &[Relation],
        _out: &mut dyn RowWriter,
    ) -> Result<(), ParseError> {
        Err(ParseError::Unsupported)
    }
    fn read_file_explicit(
        &self,
        _request: ParseRequest<'_>,
        _table: &mut Elements,
        _symbols: &mut Symbols,
        _wanted: &[Relation],
        _out: &mut dyn RowWriter,
    ) -> Result<(), ParseError> {
        Err(ParseError::Unsupported)
    }
    fn walk_file(
        &self,
        _elements: &mut Elements,
        _symbols: &mut Symbols,
        _source: ByteSlice<'_>,
        _file: FileId,
        _wanted: &[Relation],
        _out: &mut dyn RowWriter,
    ) {
    }
    fn absence_of(&self, _relation: Relation) -> Option<AbsenceCause> {
        None
    }
}

/// The actual source-backed request shared by all three refusal controls.
fn outcome_for(scenario: Scenario) -> (usize, usize) {
    let outcome = match scenario {
        Scenario::WrongOutcome => RenderOutcome::Known(RenderedNode(Arc::from("left"))),
        Scenario::NoSession => RenderOutcome::Unsupported(UnknownReason(1)),
        Scenario::UndeclaredReason => RenderOutcome::Unsupported(UnknownReason(2)),
    };
    let case = SemanticExpected {
        source: SourceFile {
            path: SourcePath("fixture.py"),
            bytes: ByteSlice(b"left"),
        },
        start: Offset(0),
        end: Offset(4),
        context: RenderContext::AttributeReceiver,
        outcome,
    };
    let mut findings = Findings(Vec::new());
    let observed = judge(&FixtureParse(scenario), &[case], &mut findings);
    (findings.0.len(), observed.len())
}

type Input = Scenario;
type Value = (usize, usize);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("wrong_expected_outcome_refuses"),
        word: Word::Succeeds,
        input: || Scenario::WrongOutcome,
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("provider_without_semantic_session_refuses"),
        word: Word::Succeeds,
        input: || Scenario::NoSession,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("undeclared_observed_reason_refuses"),
        word: Word::Succeeds,
        input: || Scenario::UndeclaredReason,
        expect: Expect::Value((1, 0)),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(outcome_for(input))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
