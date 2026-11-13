//! Target-version reader rows preserve real import identities.
use super::super::fixtures::contract::check;
use super::item::StandardLibraryImportItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::settings::{PythonParserSettings, PythonTargetVersion as Version};
use ryo::languages::{self, contract::Want};

#[derive(Clone, Copy)]
struct Input {
    version: Version,
    source: &'static str,
}
type Value = (Option<(u32, &'static str)>, bool);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("dotted_standard_root"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py310,
            source: "import os.path\n",
        },
        expect: Expect::Value((Some((4, "os.path")), false)),
    },
    Case {
        name: CaseName("tomllib_before_release"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py310,
            source: "import tomllib\n",
        },
        expect: Expect::Value((None, false)),
    },
    Case {
        name: CaseName("tomllib_after_release"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py311,
            source: "import tomllib\n",
        },
        expect: Expect::Value((Some((4, "tomllib")), false)),
    },
    Case {
        name: CaseName("distutils_before_removal"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py311,
            source: "import distutils.core\n",
        },
        expect: Expect::Value((Some((4, "distutils.core")), false)),
    },
    Case {
        name: CaseName("distutils_after_removal"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py312,
            source: "import distutils.core\n",
        },
        expect: Expect::Value((None, false)),
    },
    Case {
        name: CaseName("platform_independent_name"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py310,
            source: "import winreg\n",
        },
        expect: Expect::Value((Some((4, "winreg")), false)),
    },
    Case {
        name: CaseName("external_name"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py310,
            source: "import numpy\n",
        },
        expect: Expect::Value((None, false)),
    },
    Case {
        name: CaseName("relative_name"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py310,
            source: "from . import os\n",
        },
        expect: Expect::Value((None, false)),
    },
    Case {
        name: CaseName("unverified_older_target_is_partial"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py39,
            source: "import os\n",
        },
        expect: Expect::Value((None, true)),
    },
    Case {
        name: CaseName("future_is_standard_library_even_with_special_section"),
        word: Word::Succeeds,
        input: || Input {
            version: Version::Py310,
            source: "from __future__ import annotations\n",
        },
        expect: Expect::Value((Some((9, "__future__.annotations")), false)),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    check::<StandardLibraryImportItem>();
    let report = languages::contract::reading::<StandardLibraryImportItem>(
        &crate::linked_with_settings(PythonParserSettings::new(input.version)),
        languages::SourcePath("item.py"),
        languages::SampleText(input.source),
    )
    .expect("Python source is recognized");
    let partial = !report.causes.is_empty();
    let answer = match report.rows {
        [] => None,
        [row] => {
            let [Want::Node(node), Want::Name(target), Want::File("item.py")] = row else {
                panic!("one row with the real import node, target and file: {row:?}");
            };
            Some((*node, *target))
        }
        rows => panic!("one imported alias has at most one membership row: {rows:?}"),
    };
    Ok((answer, partial))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
