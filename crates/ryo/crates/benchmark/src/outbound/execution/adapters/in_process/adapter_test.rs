//! The in-process implementation runs a worker request through the language's worker and refuses every other invocation:
//! the shared contract's child invocations are the process implementation's, which the composition chooses for them.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::interface::ExecutionPort as _;
use crate::outbound::execution::types::{
    InProcessWorker, RuleCompileTimes, StoredRequest, WorkerResponse,
};
use benchmark_contracts::domain::measurements::types::Invocation;
use benchmark_contracts::domain::reports::errors::ReportsError;
use benchmark_contracts::domain::requests::constants::{REQUEST_FILE, RUN_RYO};
/// Which invocation runs, and whether the worker completes it.
#[derive(Clone, Copy, Debug)]
enum Worked {
    /// A worker request the worker completes.
    Completed,
    /// A worker request the worker refuses.
    Refused,
    /// A child invocation that is no worker request.
    Child,
    /// A worker request whose rules the worker cannot compile ahead.
    Uncompiled,
}
type Input = Worked;
/// The exit status, and whether stdout holds exactly the worker's response.
type Value = (i32, bool);
type Failure = ();
/// What the worker answers a request it completes with.
const RESPONSE: &[u8] = b"response";
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a completed worker request exits 0 with its response as stdout"),
        word: Word::Succeeds,
        input: || Worked::Completed,
        expect: Expect::Value((0, true)),
    },
    Case {
        name: CaseName("a refused worker request exits 1 with no response"),
        word: Word::Succeeds,
        input: || Worked::Refused,
        expect: Expect::Value((1, false)),
    },
    Case {
        name: CaseName("an invocation that is no worker request is refused"),
        word: Word::Refuses,
        input: || Worked::Child,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName(
            "a worker that cannot compile a request ahead is answered as refused, never as no compile",
        ),
        word: Word::Refuses,
        input: || Worked::Uncompiled,
        expect: Expect::Error(()),
    },
];
/// A worker that answers or refuses each request as its input says.
#[derive(Debug)]
struct Worker(Worked);
impl InProcessWorker for Worker {
    fn run(&self, _request: &StoredRequest) -> Result<WorkerResponse, ReportsError> {
        match self.0 {
            Worked::Refused => Err(ReportsError::Invalid("refused".into())),
            Worked::Completed | Worked::Child | Worked::Uncompiled => {
                Ok(WorkerResponse(RESPONSE.to_vec()))
            }
        }
    }
    fn compile(&self, _request: &StoredRequest) -> Result<Option<RuleCompileTimes>, ReportsError> {
        match self.0 {
            Worked::Uncompiled => Err(ReportsError::Invalid("uncompiled".into())),
            Worked::Completed | Worked::Refused | Worked::Child => {
                Ok(Some(RuleCompileTimes::default()))
            }
        }
    }
}
fn subject(input: Input) -> Result<Value, Failure> {
    let worker = Worker(input);
    let adapter = crate::outbound::execution::factory::in_process(&worker);
    let arguments = match input {
        Worked::Child => vec!["-c".into(), "true".into()],
        Worked::Completed | Worked::Refused | Worked::Uncompiled => {
            vec![RUN_RYO.into(), REQUEST_FILE.into(), "request.json".into()]
        }
    };
    if let Worked::Uncompiled = input {
        let invocation = Invocation {
            executable: "/bin/sh".into(),
            arguments,
            environment: std::collections::BTreeMap::new(),
            directory: std::env::temp_dir(),
        };
        // A compile answered, or none, is a run that exits 0 with no response; a refusal is the row's error.
        return adapter
            .compile(&invocation)
            .map_or(Ok((0, false)), |compiled| {
                compiled.map(|_times| (0, false)).map_err(|_refused| ())
            });
    }
    adapter
        .run(Invocation {
            executable: "/bin/sh".into(),
            arguments,
            environment: std::collections::BTreeMap::new(),
            directory: std::env::temp_dir(),
        })
        .map(|output| (output.status.0, output.stdout.0 == RESPONSE))
        .map_err(|_refused| ())
}
#[test]
fn succeeds() {
    let worker = Worker(Worked::Completed);
    crate::outbound::execution::fixtures::contract::check_with(
        &crate::outbound::execution::factory::in_process(&worker),
        &Invocation {
            executable: "/bin/sh".into(),
            arguments: vec!["-c".into(), "true".into()],
            environment: std::collections::BTreeMap::new(),
            directory: std::env::temp_dir(),
        },
    );
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
