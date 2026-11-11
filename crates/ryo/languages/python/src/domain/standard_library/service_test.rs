//! Release and platform controls for imported module roots.
use super::{
    service::contains,
    types::{ModulePath, PythonMajor, PythonMinor, StandardLibraryMembership, TargetVersion},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::settings::PythonTargetVersion as Version;

#[derive(Clone, Copy)]
struct Given {
    version: Version,
    module: &'static str,
}
/// What a row is given.
type Input = Given;
type Value = StandardLibraryMembership;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("dotted_absolute_root"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py310,
            module: "os.path",
        },
        expect: Expect::Value(StandardLibraryMembership::Standard),
    },
    Case {
        name: CaseName("tomllib_before_311"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py310,
            module: "tomllib",
        },
        expect: Expect::Value(StandardLibraryMembership::Other),
    },
    Case {
        name: CaseName("tomllib_from_311"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py311,
            module: "tomllib",
        },
        expect: Expect::Value(StandardLibraryMembership::Standard),
    },
    Case {
        name: CaseName("older_release_unverified"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py38,
            module: "graphlib",
        },
        expect: Expect::Value(StandardLibraryMembership::UnverifiedRelease),
    },
    Case {
        name: CaseName("graphlib_in_verified_release"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py310,
            module: "graphlib",
        },
        expect: Expect::Value(StandardLibraryMembership::Standard),
    },
    Case {
        name: CaseName("distutils_through_311"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py311,
            module: "distutils.core",
        },
        expect: Expect::Value(StandardLibraryMembership::Standard),
    },
    Case {
        name: CaseName("distutils_removed_312"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py312,
            module: "distutils.core",
        },
        expect: Expect::Value(StandardLibraryMembership::Other),
    },
    Case {
        name: CaseName("external_module"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py315,
            module: "numpy",
        },
        expect: Expect::Value(StandardLibraryMembership::Other),
    },
    Case {
        name: CaseName("latest_tracks_configured_parser"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Latest,
            module: "profiling",
        },
        expect: Expect::Value(StandardLibraryMembership::Standard),
    },
    Case {
        name: CaseName("empty_is_not_module"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py310,
            module: "",
        },
        expect: Expect::Value(StandardLibraryMembership::Other),
    },
    Case {
        name: CaseName("relative_is_not_absolute"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py310,
            module: ".local",
        },
        expect: Expect::Value(StandardLibraryMembership::Other),
    },
    Case {
        name: CaseName("windows_module_on_posix"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py310,
            module: "winreg",
        },
        expect: Expect::Value(StandardLibraryMembership::Standard),
    },
    Case {
        name: CaseName("posix_module_on_windows"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py310,
            module: "fcntl",
        },
        expect: Expect::Value(StandardLibraryMembership::Standard),
    },
    Case {
        name: CaseName("future_is_stdlib_even_with_special_import_section"),
        word: Word::Succeeds,
        input: || Given {
            version: Version::Py310,
            module: "__future__",
        },
        expect: Expect::Value(StandardLibraryMembership::Standard),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(contains(
        input.version.components(),
        ModulePath(input.module),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
