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
    /// A nonexecutable first PATH match followed by the native interpreter on Unix.
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
/// Process behavior to exercise.
type Input = Operation;
/// Exit status and stdout/stderr content checks.
type Value = (i32, bool, bool);
/// Classified child launch failure.
type Failure = LaunchFailure;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Launch and observation failures distinguished by the shared contract.
pub enum LaunchFailure {
    /// A portable operating-system failure category.
    Kind(std::io::ErrorKind),
    /// An invalid image is rejected by the OS or by its documented shell fallback.
    ExecutableFormat,
}
/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("fixture interpreter is native to this operating system"),
        word: Word::Succeeds,
        input: || {
            let mut call = invocation();
            call.arguments = super::driver::native();
            Operation::Call(call)
        },
        expect: Expect::Value((0_i32, true, true)),
    },
    Case {
        name: CaseName("invalid executable image is operationally refused"),
        word: Word::Refuses,
        input: || Operation::InvalidFormat,
        expect: Expect::Error(Failure::ExecutableFormat),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("missing script interpreter is a launch failure"),
        word: Word::Refuses,
        input: || Operation::MissingInterpreter,
        expect: Expect::Error(Failure::Kind(std::io::ErrorKind::NotFound)),
    },
    #[cfg(windows)]
    Case {
        name: CaseName("POSIX shebang bytes are refused as a Windows executable image"),
        word: Word::Refuses,
        input: || Operation::MissingInterpreter,
        expect: Expect::Error(Failure::ExecutableFormat),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("signal termination retains a process observation"),
        word: Word::Succeeds,
        input: || Operation::Signaled,
        expect: Expect::Value((-15_i32, false, false)),
    },
    #[cfg(windows)]
    Case {
        name: CaseName("native numeric child exit3840 does not invent a Unix signal"),
        word: Word::Succeeds,
        input: || {
            let mut call = invocation();
            call.arguments = super::driver::exiting(3840);
            Operation::Call(call)
        },
        expect: Expect::Value((3840_i32, true, true)),
    },
    Case {
        name: CaseName("both output pipes drain beyond pipe capacity"),
        word: Word::Succeeds,
        input: || Operation::LargeStreams,
        expect: Expect::Value((0_i32, true, true)),
    },
    Case {
        name: CaseName("valid child126 remains a normal exit"),
        word: Word::Succeeds,
        input: || {
            let mut call = invocation();
            call.arguments = super::driver::exiting(126);
            Operation::Call(call)
        },
        expect: Expect::Value((126_i32, true, true)),
    },
    Case {
        name: CaseName("valid child127 remains a normal exit observation"),
        word: Word::Succeeds,
        input: || {
            let mut call = invocation();
            call.arguments = super::driver::exiting(127);
            Operation::Call(call)
        },
        expect: Expect::Value((127_i32, true, true)),
    },
    #[cfg(unix)]
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
    #[cfg(unix)]
    Case {
        name: CaseName("PATH search continues past a nonexecutable shadow"),
        word: Word::Succeeds,
        input: || Operation::PathShadow,
        expect: Expect::Value((7_i32, true, true)),
    },
    Case {
        name: CaseName("a valid child exit127 is not a launch failure"),
        word: Word::Succeeds,
        input: || {
            let mut call = invocation();
            call.arguments = super::driver::exiting(127);
            Operation::Call(call)
        },
        expect: Expect::Value((127_i32, true, true)),
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
        expect: Expect::Value((7_i32, true, true)),
    },
    Case {
        name: CaseName("missing program is a launch error"),
        word: Word::Refuses,
        input: || Operation::Call(missing()),
        expect: Expect::Error(Failure::Kind(std::io::ErrorKind::NotFound)),
    },
];
/// Build the native interpreter invocation with independent stdout and stderr.
fn invocation() -> Invocation {
    Invocation {
        executable: super::driver::python().into(),
        arguments: super::driver::exiting(7),
        environment: [("RYO_BENCH_FIXTURE".into(), "payload".into())]
            .into_iter()
            .collect(),
        directory: PathBuf::from("."),
    }
}
/// Build a definitely absent executable invocation.
fn missing() -> Invocation {
    Invocation {
        executable: "/ryo-benchmark-missing-program".into(),
        arguments: Vec::new(),
        environment: std::collections::BTreeMap::default(),
        directory: PathBuf::from("."),
    }
}
/// Run the shared observations against the supplied implementation.
///
/// # Panics
///
/// Panics when fixture setup fails or the implementation violates a contract observation.
#[expect(
    clippy::expect_used,
    reason = "fixture setup failures must fail the contract test"
)]
pub fn check(adapter: &impl ExecutionPort) {
    let subject = |input| {
        let profile = adapter.profile();
        let temporary = tempfile::tempdir().expect("isolated permission fixture");
        let mut call = invocation();
        let large = matches!(input, Operation::LargeStreams);
        let invalid_image = matches!(input, Operation::InvalidFormat)
            || cfg!(windows) && matches!(input, Operation::MissingInterpreter);
        match input {
            Operation::Call(ready) => call = ready,
            Operation::NonExecutable => {
                let path = temporary.path().join("nonexecutable");
                std::fs::write(&path, "#!/bin/sh\nexit 0\n").expect("fixture file");
                call.executable = path;
            }
            Operation::Directory => {
                let path = temporary
                    .path()
                    .join(super::driver::executable_name("directory"));
                std::fs::create_dir_all(&path).expect("fixture directory");
                call.executable = path;
            }
            Operation::InvalidFormat | Operation::MissingInterpreter => {
                let format = matches!(input, Operation::InvalidFormat);
                let path = temporary
                    .path()
                    .join(super::driver::executable_name(if format {
                        "invalid-format"
                    } else {
                        "missing-interpreter"
                    }));
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
                call.arguments = super::driver::snippet(super::driver::SIGNAL);
            }
            Operation::LargeStreams => {
                call.arguments = super::driver::snippet(super::driver::LARGE_STREAMS);
            }
            Operation::PathShadow => {
                let path = temporary.path().join(super::driver::python());
                std::fs::write(&path, "not an executable").expect("shadow file");
                let existing = std::env::var_os("PATH").expect("native interpreter PATH");
                let _previous = call.environment.insert(
                    "PATH".into(),
                    std::env::join_paths(
                        std::iter::once(temporary.path().to_path_buf())
                            .chain(std::env::split_paths(&existing)),
                    )
                    .expect("fixture PATH"),
                );
            }
        }

        let output = adapter.run(call).map_err(|error| match error {
            Error::Io(error)
                if invalid_image && error.raw_os_error() == Some(super::driver::image_error()) =>
            {
                Failure::ExecutableFormat
            }
            Error::Io(error) => Failure::Kind(error.kind()),
            Error::Invalid(_) | Error::Json(_) | Error::Toml(_) => {
                Failure::Kind(std::io::ErrorKind::Other)
            }
        })?;
        // Darwin may execute its shell fallback after ENOEXEC. This classifies
        // only these independently created invalid bytes, never an arbitrary
        // child's 127 exit code (the separate valid-child case preserves it).
        if cfg!(target_os = "macos") && invalid_image {
            assert_eq!(output.status.0, 127_i32);
            assert_eq!(output.signal, None);
            assert!(output.stdout.0.is_empty());
            assert!(!output.stderr.0.is_empty());
            let _observation = observe(adapter, &profile, &output, large);
            return Err(Failure::ExecutableFormat);
        }
        Ok(observe(adapter, &profile, &output, large))
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

/// Check profile stability, resource mode, and independently specified output streams.
fn observe(
    adapter: &impl ExecutionPort,
    profile: &crate::domain::measurements::types::ExecutionProfile,
    output: &crate::domain::measurements::types::ProcessOutput,
    large: bool,
) -> Value {
    use crate::domain::measurements::types::{ExecutionProfile, ResourceObservation};
    assert_eq!(
        adapter.profile(),
        *profile,
        "execution profile must remain immutable"
    );
    assert!(
        matches!(
            (profile, output.resources),
            (ExecutionProfile::Wall, ResourceObservation::WallOnly)
                | (
                    ExecutionProfile::Direct | ExecutionProfile::Isolated(_),
                    ResourceObservation::Collected(_)
                )
        ),
        "resource observations must match the declared execution mode"
    );
    #[cfg(unix)]
    assert_eq!(
        output.signal,
        (output.status.0 < 0_i32).then_some(-output.status.0),
        "a signaled child must preserve its signal independently of measured resources"
    );
    #[cfg(not(unix))]
    assert_eq!(
        output.signal, None,
        "native numeric exit codes never invent a Unix signal"
    );
    (
        output.status.0,
        if large {
            output.stdout.0.len() == 131_072 && output.stdout.0.iter().all(|byte| *byte == 0)
        } else {
            output.stdout.0 == b"payload"
        },
        if large {
            output.stderr.0.len() == 131_072 && output.stderr.0.iter().all(|byte| *byte == 0)
        } else {
            output.stderr.0 == b"issue"
        },
    )
}
