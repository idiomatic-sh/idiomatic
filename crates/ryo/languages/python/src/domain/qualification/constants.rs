//! Source provenance labels shared by the reader and independently asserted fixtures.
pub(crate) const IMPORT: &str = "import";
pub(crate) const BUILTIN: &str = "builtin";
pub(crate) const LOCAL: &str = "local";
pub(crate) const UNRESOLVED: &str = "unresolved";
pub(crate) const AMBIGUOUS: &str = "ambiguous";
pub(crate) const BUILTIN_MODULE: &str = "builtins";
pub(crate) const DOT: &str = ".";
pub(crate) const INCOMPLETE: &str = "qualified names require a complete syntax tree";

use super::types::{Builtin, Version};
use ryo::languages::{Integer, NameText};
/// Version is inapplicable to non-builtin lexical provenance.
pub(crate) const NOT_APPLICABLE: Version = Version {
    major: Integer(0),
    minor: Integer(0),
};
/// CPython 3.15.0rc2 builtin and default-site inventory, independently captured in standard fixtures.
pub(crate) const BUILTIN_NAMES: &[NameText<'static>] = &[
    NameText("ArithmeticError"),
    NameText("AssertionError"),
    NameText("AttributeError"),
    NameText("BaseException"),
    NameText("BaseExceptionGroup"),
    NameText("BlockingIOError"),
    NameText("BrokenPipeError"),
    NameText("BufferError"),
    NameText("BytesWarning"),
    NameText("ChildProcessError"),
    NameText("ConnectionAbortedError"),
    NameText("ConnectionError"),
    NameText("ConnectionRefusedError"),
    NameText("ConnectionResetError"),
    NameText("DeprecationWarning"),
    NameText("EOFError"),
    NameText("Ellipsis"),
    NameText("EncodingWarning"),
    NameText("EnvironmentError"),
    NameText("Exception"),
    NameText("ExceptionGroup"),
    NameText("False"),
    NameText("FileExistsError"),
    NameText("FileNotFoundError"),
    NameText("FloatingPointError"),
    NameText("FutureWarning"),
    NameText("GeneratorExit"),
    NameText("IOError"),
    NameText("ImportCycleError"),
    NameText("ImportError"),
    NameText("ImportWarning"),
    NameText("IndentationError"),
    NameText("IndexError"),
    NameText("InterruptedError"),
    NameText("IsADirectoryError"),
    NameText("KeyError"),
    NameText("KeyboardInterrupt"),
    NameText("LookupError"),
    NameText("MemoryError"),
    NameText("ModuleNotFoundError"),
    NameText("NameError"),
    NameText("None"),
    NameText("NotADirectoryError"),
    NameText("NotImplemented"),
    NameText("NotImplementedError"),
    NameText("OSError"),
    NameText("OverflowError"),
    NameText("PendingDeprecationWarning"),
    NameText("PermissionError"),
    NameText("ProcessLookupError"),
    NameText("PythonFinalizationError"),
    NameText("RecursionError"),
    NameText("ReferenceError"),
    NameText("ResourceWarning"),
    NameText("RuntimeError"),
    NameText("RuntimeWarning"),
    NameText("StopAsyncIteration"),
    NameText("StopIteration"),
    NameText("SyntaxError"),
    NameText("SyntaxWarning"),
    NameText("SystemError"),
    NameText("SystemExit"),
    NameText("TabError"),
    NameText("TimeoutError"),
    NameText("True"),
    NameText("TypeError"),
    NameText("UnboundLocalError"),
    NameText("UnicodeDecodeError"),
    NameText("UnicodeEncodeError"),
    NameText("UnicodeError"),
    NameText("UnicodeTranslateError"),
    NameText("UnicodeWarning"),
    NameText("UserWarning"),
    NameText("ValueError"),
    NameText("Warning"),
    NameText("ZeroDivisionError"),
    NameText("__build_class__"),
    NameText("__debug__"),
    NameText("__import__"),
    NameText("abs"),
    NameText("aiter"),
    NameText("all"),
    NameText("anext"),
    NameText("any"),
    NameText("ascii"),
    NameText("bin"),
    NameText("bool"),
    NameText("breakpoint"),
    NameText("bytearray"),
    NameText("bytes"),
    NameText("callable"),
    NameText("chr"),
    NameText("classmethod"),
    NameText("compile"),
    NameText("complex"),
    NameText("copyright"),
    NameText("credits"),
    NameText("delattr"),
    NameText("dict"),
    NameText("dir"),
    NameText("divmod"),
    NameText("enumerate"),
    NameText("eval"),
    NameText("exec"),
    NameText("exit"),
    NameText("filter"),
    NameText("float"),
    NameText("format"),
    NameText("frozendict"),
    NameText("frozenset"),
    NameText("getattr"),
    NameText("globals"),
    NameText("hasattr"),
    NameText("hash"),
    NameText("help"),
    NameText("hex"),
    NameText("id"),
    NameText("input"),
    NameText("int"),
    NameText("isinstance"),
    NameText("issubclass"),
    NameText("iter"),
    NameText("len"),
    NameText("license"),
    NameText("list"),
    NameText("locals"),
    NameText("map"),
    NameText("max"),
    NameText("memoryview"),
    NameText("min"),
    NameText("next"),
    NameText("object"),
    NameText("oct"),
    NameText("open"),
    NameText("ord"),
    NameText("pow"),
    NameText("print"),
    NameText("property"),
    NameText("quit"),
    NameText("range"),
    NameText("repr"),
    NameText("reversed"),
    NameText("round"),
    NameText("sentinel"),
    NameText("set"),
    NameText("setattr"),
    NameText("slice"),
    NameText("sorted"),
    NameText("staticmethod"),
    NameText("str"),
    NameText("sum"),
    NameText("super"),
    NameText("tuple"),
    NameText("type"),
    NameText("vars"),
    NameText("zip"),
];
/// Names introduced after Python 3.0, from the Python library version annotations.
pub(crate) const ADDED_BUILTINS: &[Builtin] = &[
    Builtin {
        name: NameText("callable"),
        since: Version {
            major: Integer(3),
            minor: Integer(2),
        },
    },
    Builtin {
        name: NameText("BaseExceptionGroup"),
        since: Version {
            major: Integer(3),
            minor: Integer(11),
        },
    },
    Builtin {
        name: NameText("BlockingIOError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("BrokenPipeError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("ChildProcessError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("ConnectionAbortedError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("ConnectionError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("ConnectionRefusedError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("ConnectionResetError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("EncodingWarning"),
        since: Version {
            major: Integer(3),
            minor: Integer(10),
        },
    },
    Builtin {
        name: NameText("ExceptionGroup"),
        since: Version {
            major: Integer(3),
            minor: Integer(11),
        },
    },
    Builtin {
        name: NameText("FileExistsError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("FileNotFoundError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("ImportCycleError"),
        since: Version {
            major: Integer(3),
            minor: Integer(15),
        },
    },
    Builtin {
        name: NameText("InterruptedError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("IsADirectoryError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("ModuleNotFoundError"),
        since: Version {
            major: Integer(3),
            minor: Integer(6),
        },
    },
    Builtin {
        name: NameText("NotADirectoryError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("PermissionError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("ProcessLookupError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("PythonFinalizationError"),
        since: Version {
            major: Integer(3),
            minor: Integer(13),
        },
    },
    Builtin {
        name: NameText("RecursionError"),
        since: Version {
            major: Integer(3),
            minor: Integer(5),
        },
    },
    Builtin {
        name: NameText("ResourceWarning"),
        since: Version {
            major: Integer(3),
            minor: Integer(2),
        },
    },
    Builtin {
        name: NameText("StopAsyncIteration"),
        since: Version {
            major: Integer(3),
            minor: Integer(5),
        },
    },
    Builtin {
        name: NameText("TimeoutError"),
        since: Version {
            major: Integer(3),
            minor: Integer(3),
        },
    },
    Builtin {
        name: NameText("aiter"),
        since: Version {
            major: Integer(3),
            minor: Integer(10),
        },
    },
    Builtin {
        name: NameText("anext"),
        since: Version {
            major: Integer(3),
            minor: Integer(10),
        },
    },
    Builtin {
        name: NameText("breakpoint"),
        since: Version {
            major: Integer(3),
            minor: Integer(7),
        },
    },
    Builtin {
        name: NameText("frozendict"),
        since: Version {
            major: Integer(3),
            minor: Integer(15),
        },
    },
    Builtin {
        name: NameText("sentinel"),
        since: Version {
            major: Integer(3),
            minor: Integer(15),
        },
    },
];
/// Default availability for the remaining Python 3 builtin namespace.
pub(crate) const PYTHON_THREE: Version = Version {
    major: Integer(3),
    minor: Integer(0),
};
