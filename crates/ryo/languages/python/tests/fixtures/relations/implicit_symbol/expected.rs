// The builtin inventory comes from the pinned independent CPython standard fixture.
// Environment origins and introduction versions are distinct from an occurrence's availability.
use ryo::languages::contract::{
    Expected, ImplicitSymbolExpectedRow, expected_rows, file, name, node, number,
};

/// Independent public builtin inventory, captured with CPython -I (see standard/README.md).
const BUILTINS: &str = include_str!("../../standard/builtin_names.txt");

/// Versioned environment names do not invent a declaration occurrence.
fn row(spelling: &'static str, kind: &'static str, minor: i64) -> ImplicitSymbolExpectedRow {
    ImplicitSymbolExpectedRow {
        name: name(spelling),
        kind: name(kind),
        scope: node(0),
        since_major: number(3),
        since_minor: number(minor),
        file: file("implicit_symbol/input.py"),
    }
}

/// Introduction versions from the Python library's functions/exceptions version annotations.
fn introduced(spelling: &str) -> i64 {
    match spelling {
        "callable" | "ResourceWarning" => 2,
        "BlockingIOError"
        | "BrokenPipeError"
        | "ChildProcessError"
        | "ConnectionAbortedError"
        | "ConnectionError"
        | "ConnectionRefusedError"
        | "ConnectionResetError"
        | "FileExistsError"
        | "FileNotFoundError"
        | "InterruptedError"
        | "IsADirectoryError"
        | "NotADirectoryError"
        | "PermissionError"
        | "ProcessLookupError"
        | "TimeoutError" => 3,
        "RecursionError" | "StopAsyncIteration" => 5,
        "ModuleNotFoundError" => 6,
        "breakpoint" => 7,
        "aiter" | "anext" | "EncodingWarning" => 10,
        "BaseExceptionGroup" | "ExceptionGroup" => 11,
        "PythonFinalizationError" => 13,
        "ImportCycleError" | "frozendict" | "sentinel" => 15,
        _ => 0,
    }
}

/// A potential catalog is independent of reads, bindings, target filtering, and initialization.
pub(super) fn expected() -> Expected {
    let mut rows: Vec<_> = BUILTINS
        .lines()
        .map(|spelling| row(spelling, "builtin", introduced(spelling)))
        .collect();
    rows.extend([
        row("__doc__", "builtin", 0),
        row("__loader__", "builtin", 0),
        row("__name__", "builtin", 0),
        row("__package__", "builtin", 0),
        row("__spec__", "builtin", 4),
        row("__annotations__", "module-global", 6),
        row("__builtins__", "module-global", 0),
        row("__cached__", "module-global", 2),
        row("__doc__", "module-global", 0),
        row("__file__", "module-global", 0),
        row("__loader__", "module-global", 0),
        row("__name__", "module-global", 0),
        row("__package__", "module-global", 0),
        row("__spec__", "module-global", 4),
        row("__warningregistry__", "module-global", 0),
        row("__annotate__", "module-global", 14),
        row("WindowsError", "platform-builtin", 0),
    ]);
    expected_rows(&rows)
}
