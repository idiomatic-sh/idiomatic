//! The node counters of a run, through the pass's program: a subscriber that enables their target switches the counting on
//! for the files the program runs, and dropping the program emits one `node` event per node made, with its kind and the
//! rules whose steps it is, then one `run` event; a subscriber that does not enable it is sent nothing.
use crate::application::file_dag::file_pass::constants::NODE_COUNTERS_TARGET;
use crate::application::file_dag::file_pass::service::{given_back, pass_program, taken};
use crate::application::file_dag::file_pass::types::PassSource;
use crate::outbound::operators::fixtures::fake::OperatorsFake;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::service::{physical, rule};
use ryo_operators::domain::physical_plan::fixtures::defines_rows;
use ryo_operators::inbound::use_cases::run::fixtures::{TableInputs, TextResources};
use ryo_relations::domain::relations::types::{Integer, Offset, Relation, Span, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput, StepId, StepScope};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::runs::types::RuleId;
use std::borrow::Cow;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

/// Whether the run's subscriber enables the counters' target.
#[derive(Clone, Copy, Debug)]
enum Listening {
    /// It does.
    On,
    /// The planted counter-example: it does not.
    Off,
}

type Input = Listening;
/// The events the subscriber was sent, one per line: the event's name and its fields, the times left out.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_run_its_subscriber_listens_to_emits_each_node_made_and_the_run"),
        word: Word::Succeeds,
        input: || Listening::On,
        // The two rules' one node, made once in each file of three rows, every row kept; then the run.
        expect: Expect::Value(Cow::Borrowed(
            "node node=0 kind=Filter runs=2 empty_runs=0 rows_in=6 rows_out=6 \
             rules=[RuleId(\"first\"), RuleId(\"second\")]\n\
             run nodes=1 runs=2 empty_runs=0 rows_in=6 rows_out=6",
        )),
    },
    Case {
        name: CaseName("a_run_its_subscriber_does_not_listen_to_emits_nothing"),
        word: Word::Succeeds,
        input: || Listening::Off,
        expect: Expect::Value(Cow::Borrowed("")),
    },
];

/// A subscriber that keeps the events of the counters' target, when it listens to them, as text.
#[derive(Debug)]
struct Events {
    /// Whether it enables the counters' target.
    listening: bool,
    /// The events it was sent, one line each.
    lines: Arc<Mutex<Vec<String>>>,
}

/// Writes each field of an event, its times left out, which no run repeats.
struct Fields<'a>(&'a mut String);

impl tracing::field::Visit for Fields<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if !field.name().ends_with("nanos") && field.name() != "empty_share" {
            let _written = write!(self.0, " {}={value:?}", field.name());
        }
    }
}

impl tracing::Subscriber for Events {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        self.listening && metadata.target() == NODE_COUNTERS_TARGET
    }
    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut line = event.metadata().name().to_owned();
        event.record(&mut Fields(&mut line));
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// A per-file plan that keeps every `Defines` row, emitted.
fn plan() -> Arc<Plan> {
    Arc::new(Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        frontier: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        steps: vec![PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: StepInput::Relation(Relation::Defines),
                comparisons: Vec::new(),
                kept: Vec::new(),
            },
        }],
    })
}

/// Two rules of the same plan, so their one node is both rules', run over two files of three rows each, under a subscriber
/// that listens to the counters when `listening` says; answers what it was sent.
fn subject(listening: Input) -> Result<Value, Failure> {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let events = Events {
        listening: matches!(listening, Listening::On),
        lines: Arc::clone(&lines),
    };
    let source = PassSource {
        plans: vec![plan(), plan()],
        bindings: vec![Vec::new(), Vec::new()],
        operators: Arc::new(OperatorsFake::default()),
        hosted: Vec::new(),
        ids: vec![RuleId("first".into()), RuleId("second".into())],
    };
    let mut texts = TextResources::new(&[]);
    let span = Span {
        start: Offset(0),
        end: Offset(1),
    };
    let rows = defines_rows(&[(Cell::Number(Integer(0)), span); 3]);
    let inputs = TableInputs::new(&[(Relation::Defines, &rows)]);
    tracing::subscriber::with_default(events, || -> Result<(), Failure> {
        let pass = pass_program(&source).map_err(|_| ())?;
        let node = rule(&pass.program, RuleIndex(0))
            .map_err(|_| ())?
            .emit
            .clone()
            .map_err(|_| ())?;
        for _file in 0..2 {
            let mut scratch = taken(&pass);
            pass.operators
                .run(
                    physical(&pass.program),
                    node,
                    &mut scratch.buffers,
                    &inputs,
                    &mut texts,
                )
                .map_err(|_| ())?;
            scratch.buffers.release(node);
            given_back(&pass, scratch);
        }
        drop(pass);
        Ok(())
    })?;
    let lines = lines.lock().unwrap_or_else(PoisonError::into_inner);
    Ok(Cow::Owned(lines.join("\n")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
