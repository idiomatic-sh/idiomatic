//! Proves the next step each error gives.

use super::{Next, Remedy as _};
use asymptote_core::domain::selection::types::At;
use asymptote_core::domain::violations::types::{LineNumber, RepoPath};
use asymptote_core::inbound::use_cases::check::errors::CheckError;
use asymptote_core::inbound::use_cases::down::errors::DownError;
use asymptote_core::inbound::use_cases::up::errors::UpError;
use asymptote_core::outbound::manifest::errors::ManifestError;

/// A command line to run is said as one.
#[test]
fn a_run_succeeds_as_the_command_in_backticks() {
    assert_eq!(
        Next::Run("asymptote check".to_owned()).to_string(),
        "run `asymptote check`"
    );
}

/// A stale report sends the user back to `check`.
#[test]
fn a_stale_report_refuses_with_check_again() {
    assert_eq!(
        UpError::HeadMoved.next(),
        Next::Run("asymptote check".to_owned())
    );
    assert_eq!(
        DownError::NoReport.next(),
        Next::Run("asymptote check".to_owned())
    );
}

/// No product says where one is looked for.
#[test]
fn no_product_refuses_with_where_to_run() {
    let error = CheckError::Manifest(ManifestError::Missing {
        start: "/x".to_owned(),
    });
    assert!(matches!(error.next(), Next::Fix(said) if said.contains("--root")));
}

/// An `--at` nothing is at names the nearest new violations.
#[test]
fn nothing_at_a_place_refuses_with_the_nearest() {
    let error = UpError::NothingAt {
        at: At {
            path: RepoPath("a.rs".into()),
            line: LineNumber(9),
        },
        nearest: Vec::new(),
    };
    assert!(matches!(error.next(), Next::Fix(said) if said.contains("nothing is new")));
}

/// A workspace member that cannot be one says how to make it one.
#[test]
fn a_member_that_cannot_be_one_refuses_with_what_a_member_is() {
    let error = CheckError::Manifest(ManifestError::Member {
        member: "crates/c".to_owned(),
        reason: "it has no asymptote.toml".to_owned(),
    });
    assert!(matches!(error.next(), Next::Fix(said) if said.contains("of their own")));
    let nested = CheckError::Manifest(ManifestError::NotRoot {
        path: "/p/inner/asymptote.toml".to_owned(),
    });
    assert!(matches!(nested.next(), Next::Fix(said) if said.contains("only the workspace root")));
}

/// A repository that needs another asymptote says how to install it.
#[test]
fn another_asymptote_than_the_root_names_refuses_with_the_installer() {
    let error = CheckError::Manifest(ManifestError::Pinned {
        needed: "0.0.1".to_owned(),
        installed: "0.1.0".to_owned(),
    });
    let said = error.next().to_string();
    assert!(
        said.contains("tools/install.py") && !said.contains("cargo install"),
        "{said}"
    );
}
