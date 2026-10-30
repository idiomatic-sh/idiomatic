//! Proves what the user does next about each error a command can end with.

use super::{Next, Remedy as _};
use tastemaker_core::inbound::use_cases::check::errors::CheckError;

/// A command line that moves the user on is printed as the command to run.
#[test]
fn next_run_succeeds_as_the_command_to_run() {
    assert_eq!(
        Next::Run("tm check --fix".to_owned()).to_string(),
        "run `tm check --fix`"
    );
}

/// What to change is printed as the instruction it is.
#[test]
fn next_fix_succeeds_as_the_instruction() {
    assert_eq!(
        Next::Fix("pass `--package <dir>`".to_owned()).to_string(),
        "pass `--package <dir>`"
    );
}

/// A bug is printed with where to report it.
#[test]
fn next_report_succeeds_with_where_to_report() {
    let said = Next::Report.to_string();
    assert!(said.contains("this is a bug"), "{said}");
    assert!(said.contains("/issues/new"), "{said}");
}

/// A stale lock is fixed by the check that rewrites it.
#[test]
fn a_stale_lock_refuses_with_check_fix() {
    let error = CheckError::Lock {
        file: "tastemaker.lock".to_owned(),
        state: "stale".to_owned(),
    };
    assert_eq!(error.next(), Next::Run("tm check --fix".to_owned()));
}

/// A root that names another Tastemaker is fixed by installing that version: with the installer of the repository that declares it,
/// else as the README says.
#[test]
fn another_tastemaker_than_the_root_names_refuses_with_the_version_to_install() {
    let error = CheckError::Manifest {
        file: "tastemaker.toml".to_owned(),
        source: tastemaker_core::outbound::manifest::errors::ManifestError::Needs {
            needs: "9.9.9".to_owned(),
            installed: "0.1.0".to_owned(),
        },
    };
    let said = error.next().to_string();
    assert!(
        said.contains("install tastemaker 9.9.9") && said.contains("tools/install.py"),
        "{said}"
    );
}
