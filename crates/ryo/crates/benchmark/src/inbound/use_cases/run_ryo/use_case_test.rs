//! Receive, engine and response failures stop the pipeline at the failed boundary.
use super::{interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::{errors::DiagnosticsError as Error, types::RunReport},
    suite::types::WorkerRequest,
};
use crate::outbound::{
    engine::interface::EnginePort,
    worker::fixtures::{data::request, fake::WorkerFake},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[derive(Clone, Copy, Debug)]
enum Stage {
    Complete,
    Receive,
    Engine,
    Respond,
}
type Input = Stage;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Trace {
    received: usize,
    evaluated: usize,
    responded: usize,
    published: bool,
}
type Value = Trace;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Receive(Trace),
    Engine(Trace),
    Respond(Trace),
    Unexpected,
}
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("receive engine respond publishes the actual report"),
        word: Word::Succeeds,
        input: || Stage::Complete,
        expect: Expect::Value(Trace {
            received: 1,
            evaluated: 1,
            responded: 1,
            published: true,
        }),
    },
    Case {
        name: CaseName("receive failure prevents engine and response"),
        word: Word::Fails,
        input: || Stage::Receive,
        expect: Expect::Error(Refusal::Receive(Trace {
            received: 1,
            evaluated: 0,
            responded: 0,
            published: false,
        })),
    },
    Case {
        name: CaseName("engine failure prevents response"),
        word: Word::Fails,
        input: || Stage::Engine,
        expect: Expect::Error(Refusal::Engine(Trace {
            received: 1,
            evaluated: 1,
            responded: 0,
            published: false,
        })),
    },
    Case {
        name: CaseName("response failure cannot report success"),
        word: Word::Fails,
        input: || Stage::Respond,
        expect: Expect::Error(Refusal::Respond(Trace {
            received: 1,
            evaluated: 1,
            responded: 1,
            published: false,
        })),
    },
];
#[derive(Debug)]
struct Engine {
    calls: AtomicUsize,
    refuses: bool,
}
impl EnginePort for Engine {
    fn run(&self, request: WorkerRequest) -> Result<RunReport, Error> {
        let _previous = self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuses {
            return Err(Error::Invalid("fixture engine failure".into()));
        }
        Ok(RunReport {
            files: request.files,
            diagnostics: Vec::new(),
        })
    }
}
fn subject(stage: Input) -> Result<Value, Failure> {
    let mut transport = WorkerFake::new(request());
    transport.refuse_receive = matches!(stage, Stage::Receive);
    transport.refuse_respond = matches!(stage, Stage::Respond);
    let transport = Arc::new(transport);
    let engine = Arc::new(Engine {
        calls: AtomicUsize::new(0),
        refuses: matches!(stage, Stage::Engine),
    });
    let result = RunRyoUseCaseImpl::new(
        Arc::<Engine>::clone(&engine),
        Arc::<WorkerFake>::clone(&transport),
    )
    .run_ryo();
    let trace = Trace {
        received: transport.receive_calls(),
        evaluated: engine.calls.load(Ordering::SeqCst),
        responded: transport.respond_calls(),
        published: transport.reports()
            == [RunReport {
                files: request().files,
                diagnostics: Vec::new(),
            }],
    };
    result.map(|()| trace).map_err(|error| match error {
        Error::Invalid(message) if message == "fixture receive failure" => Refusal::Receive(trace),
        Error::Invalid(message) if message == "fixture engine failure" => Refusal::Engine(trace),
        Error::Io(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
            Refusal::Respond(trace)
        }
        Error::Invalid(_) | Error::Io(_) | Error::Json(_) | Error::Toml(_) => Refusal::Unexpected,
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
