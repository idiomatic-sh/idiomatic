//! Execution preserves exit status and both streams; launch failure is distinct.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::measurements::types::Invocation;
use crate::outbound::execution::interface::ExecutionPort;
use std::path::PathBuf;
/// Execution requests and independently materialized launch failures.
#[derive(Debug)]
pub enum Operation {
    /// Run a prepared child invocation.
    Call(Invocation),
    /// A regular file created without executable permission.
    NonExecutable,
    /// A directory supplied as an executable.
    Directory,
    /// A nonexecutable first PATH match followed by a real shell.
    PathShadow,
    /// Executable bytes with no recognized image or script header.
    InvalidFormat,
    /// A script whose interpreter does not exist.
    MissingInterpreter,
    /// A child terminated by a signal, with no ordinary exit code.
    Signaled,
    /// Both pipes produce more bytes than their kernel buffers hold.
    LargeStreams,
}
type Input = Operation;
type Value = (i32, bool, bool);
type Failure = LaunchFailure;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Launch and observation failures distinguished by the shared contract.
pub enum LaunchFailure {
    /// A portable operating-system failure category.
    Kind(std::io::ErrorKind),
    /// Linux ENOEXEC from an unrecognized executable image.
    ExecutableFormat,
}
/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("invalid executable image is a launch failure"),
        word: Word::Refuses,
        input: || Operation::InvalidFormat,
        expect: Expect::Error(Failure::ExecutableFormat),
    },
    Case {
        name: CaseName("missing script interpreter is a launch failure"),
        word: Word::Refuses,
        input: || Operation::MissingInterpreter,
        expect: Expect::Error(Failure::Kind(std::io::ErrorKind::NotFound)),
    },
    Case {
        name: CaseName("signal termination is not a successful observation"),
        word: Word::Refuses,
        input: || Operation::Signaled,
        expect: Expect::Error(Failure::Kind(std::io::ErrorKind::Other)),
    },
    Case {
        name: CaseName("both output pipes drain beyond pipe capacity"),
        word: Word::Succeeds,
        input: || Operation::LargeStreams,
        expect: Expect::Value((0, true, true)),
    },
    Case {
        name: CaseName("valid child126 remains a normal exit"),
        word: Word::Succeeds,
        input: || {
            let mut call = invocation();
            call.arguments[1] =
                "printf %s \"$RYO_BENCH_FIXTURE\"; printf issue >&2; exit 126".into();
            Operation::Call(call)
        },
        expect: Expect::Value((126, true, true)),
    },
    Case {
        name: CaseName("a regular file without execute permission is a launch error"),
        word: Word::Refuses,
        input: || Operation::NonExecutable,
        expect: Expect::Error(Failure::Kind(std::io::ErrorKind::PermissionDenied)),
    },
    Case {
        name: CaseName("a directory cannot be launched"),
        word: Word::Refuses,
        input: || Operation::Directory,
        expect: Expect::Error(Failure::Kind(std::io::ErrorKind::PermissionDenied)),
    },
    Case {
        name: CaseName("PATH search continues past a nonexecutable shadow"),
        word: Word::Succeeds,
        input: || Operation::PathShadow,
        expect: Expect::Value((7, true, true)),
    },
    Case {
        name: CaseName("a valid child exit127 is not a launch failure"),
        word: Word::Succeeds,
        input: || {
            let mut call = invocation();
            call.arguments[1] =
                "printf %s \"$RYO_BENCH_FIXTURE\"; printf issue >&2; exit 127".into();
            Operation::Call(call)
        },
        expect: Expect::Value((127, true, true)),
    },
    Case {
        name: CaseName("missing bare command is a launch error"),
        word: Word::Refuses,
        input: || {
            let mut call = missing();
            call.executable = "ryo-benchmark-missing-program".into();
            Operation::Call(call)
        },
        expect: Expect::Error(Failure::Kind(std::io::ErrorKind::NotFound)),
    },
    Case {
        name: CaseName("status and both streams survive"),
        word: Word::Succeeds,
        input: || Operation::Call(invocation()),
        expect: Expect::Value((7, true, true)),
    },
    Case {
        name: CaseName("missing program is a launch error"),
        word: Word::Refuses,
        input: || Operation::Call(missing()),
        expect: Expect::Error(Failure::Kind(std::io::ErrorKind::NotFound)),
    },
];
fn invocation() -> Invocation {
    Invocation {
        executable: "sh".into(),
        arguments: vec![
            "-c".into(),
            "printf %s \"$RYO_BENCH_FIXTURE\"; printf issue >&2; exit 7".into(),
        ],
        environment: [("RYO_BENCH_FIXTURE".into(), "payload".into())]
            .into_iter()
            .collect(),
        directory: PathBuf::from("."),
    }
}
fn missing() -> Invocation {
    Invocation {
        executable: "/ryo-benchmark-missing-program".into(),
        arguments: Vec::new(),
        environment: Default::default(),
        directory: PathBuf::from("."),
    }
}
/// Run the shared observations against the supplied implementation.
///
/// # Panics
///
/// Panics when fixture setup fails or the implementation violates a contract observation.
pub fn check(adapter: &impl ExecutionPort) {
    let subject = |input| {
        let temporary = tempfile::tempdir().expect("isolated permission fixture");
        let mut call = invocation();
        let large = matches!(input, Operation::LargeStreams);
        match input {
            Operation::Call(ready) => call = ready,
            Operation::NonExecutable => {
                let path = temporary.path().join("nonexecutable");
                std::fs::write(&path, "#!/bin/sh\nexit 0\n").expect("fixture file");
                call.executable = path;
            }
            Operation::Directory => {
                let path = temporary.path().join("directory");
                std::fs::create_dir(&path).expect("fixture directory");
                call.executable = path;
            }
            Operation::InvalidFormat | Operation::MissingInterpreter => {
                let format = matches!(input, Operation::InvalidFormat);
                let path = temporary.path().join(if format {
                    "invalid-format"
                } else {
                    "missing-interpreter"
                });
                std::fs::write(
                    &path,
                    if format {
                        "not executable image bytes"
                    } else {
                        "#!/ryo-benchmark-missing-interpreter\n"
                    },
                )
                .expect("executable fixture");
                super::driver::make_executable(&path).expect("executable permission fixture");
                call.executable = path;
            }
            Operation::Signaled => {
                call.arguments[1] = "kill -TERM $$".into();
            }
            Operation::LargeStreams => {
                call.arguments[1] = "head -c 131072 /dev/zero; head -c 131072 /dev/zero >&2".into();
            }
            Operation::PathShadow => {
                let path = temporary.path().join("sh");
                std::fs::write(&path, "not an executable").expect("shadow file");
                let _previous = call.environment.insert(
                    "PATH".into(),
                    std::env::join_paths([
                        temporary.path(),
                        std::path::Path::new("/usr/bin"),
                        std::path::Path::new("/bin"),
                    ])
                    .expect("fixture PATH"),
                );
            }
        }

        adapter
            .run(call)
            .map(|output| {
                (
                    output.status.0,
                    if large {
                        output.stdout.0.len() == 131072
                            && output.stdout.0.iter().all(|byte| *byte == 0)
                    } else {
                        output.stdout.0 == b"payload"
                    },
                    if large {
                        output.stderr.0.len() == 131072
                            && output.stderr.0.iter().all(|byte| *byte == 0)
                    } else {
                        output.stderr.0 == b"issue"
                    },
                )
            })
            .map_err(|error| match error {
                Error::Io(error) if error.raw_os_error() == Some(8) => Failure::ExecutableFormat,
                Error::Io(error) => Failure::Kind(error.kind()),
                _ => Failure::Kind(std::io::ErrorKind::Other),
            })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
