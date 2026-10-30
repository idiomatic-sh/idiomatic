//! The requirements file's cases: names, markers, comments, includes, constraints, paths and URLs.

use super::declared;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::fixtures::shown::{Files, declared_with};

/// The manifests of a case: the first is read.
type Input = Files;
/// What it declares, as one line.
type Value = &'static str;
type Failure = ManifestReadingError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_line_is_a_requirement_with_its_extras_and_its_marker_as_written"),
        word: Word::Succeeds,
        input: || {
            &[(
                "requirements.txt",
                "requests[security]>=2 ; python_version >= '3.9'\nFlask==2.0 --hash=sha256:abc\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | requests[security]>=2 ; python_version >= '3.9' Normal Required <>=2>, Flask==2.0 Normal Required <==2.0>!",
        ),
    },
    Case {
        name: CaseName("comments_blank_lines_options_and_continuations_are_not_requirements"),
        word: Word::Succeeds,
        input: || {
            &[(
                "requirements.txt",
                "# pinned\n\n--index-url https://example.com/simple\n-c constraints.txt\n--pre\nnumpy>=1 # fast\\\n\nscipy \\\n  >=1\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | numpy>=1 Normal Required <>=1>, scipy   >=1 Normal Required <>=1>",
        ),
    },
    Case {
        name: CaseName("a_development_file_holds_development_requirements"),
        word: Word::Succeeds,
        input: || &[("requirements-dev.txt", "pytest\n")],
        expect: Expect::Value(" |  |  |  | pytest Dev Required"),
    },
    Case {
        name: CaseName(
            "an_included_file_is_read_relative_to_the_file_that_includes_it_with_its_own_kind",
        ),
        word: Word::Succeeds,
        input: || {
            &[
                (
                    "requirements/dev.txt",
                    "-r base.txt\n--requirement=../extra.txt\npytest\n",
                ),
                ("requirements/base.txt", "django\n"),
                ("extra.txt", "-r requirements/dev.txt\nrich\n"),
            ]
        },
        expect: Expect::Value(
            " |  |  |  | django Normal Required, rich Normal Required, pytest Dev Required",
        ),
    },
    Case {
        name: CaseName("an_include_of_a_file_the_repository_did_not_hand_over_is_refused"),
        word: Word::Refuses,
        input: || &[("requirements.txt", "-r base.txt\n")],
        expect: Expect::Refusal(|error| {
            crate::cases::selects(
                matches!(error, ManifestReadingError::Unreadable { reason } if reason.contains("base.txt")),
            )
        }),
    },
    Case {
        name: CaseName("a_path_is_the_directory_of_a_package_from_the_directory_of_the_file"),
        word: Word::Succeeds,
        input: || {
            &[(
                "apps/requirements.txt",
                "-e ../libs/lib\n./vendor/other[extra]\n-e .\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  |  Normal Required @libs/lib Path,  Normal Required @apps/vendor/other Path,  Normal Required @apps Path",
        ),
    },
    Case {
        name: CaseName("a_url_names_no_package_of_the_repository_unless_it_gives_a_name"),
        word: Word::Succeeds,
        input: || {
            &[(
                "requirements.txt",
                "https://example.com/pkg.tar.gz\ngit+https://example.com/x.git#egg=x\nlib @ https://example.com/lib.whl\n./dist/pkg.whl\n/abs/path\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  |  Normal Required @? Url, x Normal Required Git, lib @ https://example.com/lib.whl Normal Required Url,  Normal Required @? Path,  Normal Required @? Path",
        ),
    },
    Case {
        name: CaseName("a_file_of_options_and_constraints_alone_declares_nothing"),
        word: Word::Succeeds,
        input: || {
            &[(
                "requirements.txt",
                "-c constraints.txt\n--extra-index-url https://example.com\n",
            )]
        },
        expect: Expect::Value(" |  |  |  | "),
    },
    Case {
        name: CaseName(
            "a_line_says_its_registry_its_path_or_its_git_repository_and_the_revision_its_url_names_after_its_path",
        ),
        word: Word::Succeeds,
        input: || {
            &[(
                "requirements.txt",
                "a==1\n./libs/b ; sys_platform == 'win32'\ngit+https://example.com/c.git@main#egg=c\nd @ git+ssh://git@example.com/d.git@v2.0\n",
            )]
        },
        expect: Expect::Value(
            " |  |  |  | a==1 Normal Required <==1>!,  ; sys_platform == 'win32' Normal Required @libs/b Path, c Normal Required Git rev=main, d @ git+ssh://git@example.com/d.git@v2.0 Normal Required Git rev=v2.0",
        ),
    },
];

fn subject(files: Input) -> Result<Value, Failure> {
    declared_with(declared, files).map(|line| &*Box::leak(line.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
