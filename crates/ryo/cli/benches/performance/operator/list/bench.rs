//! `operator list`'s workload.

/// `operator list`, run once per round, which must succeed.
pub(crate) fn workload() -> crate::measure::Measured {
    crate::measure::rounds(
        "operator/list",
        || (),
        |()| {
            let line = <ryo_cli::resources::operator::list::command::ListArguments as clap::Args>::augment_args(
                clap::Command::new("list"),
            );
            let arguments = <ryo_cli::resources::operator::list::command::ListArguments as clap::FromArgMatches>::from_arg_matches(
                &line.get_matches_from(["list"]),
            );
            assert!(
                arguments.is_ok(),
                "`operator list` parses with no arguments"
            );
            let Ok(arguments) = arguments else { return };
            let outcome = ryo_cli::resources::operator::list::command::run(
                arguments,
                &crate::measure::quiet_edge("."),
            );
            assert_eq!(
                outcome,
                ryo_cli::outcome::CommandOutcome::Success,
                "`operator list` did not succeed"
            );
        },
    )
}
